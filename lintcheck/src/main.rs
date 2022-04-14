// Run clippy on a fixed set of crates and collect the warnings.
// This helps observing the impact clippy changes have on a set of real-world code (and not just our
// testsuite).
//
// When a new lint is introduced, we can search the results for new warnings and check for false
// positives.

#![feature(let_else, absolute_path)]
#![allow(clippy::collapsible_else_if)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::io::{self, ErrorKind};
use std::path::{absolute, Path, PathBuf};
use std::process::{Command, Stdio};
use std::{cmp, env, fs};

use clap::{App, Arg, ArgMatches};
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(not(windows))]
const CLIPPY_DRIVER_PATH: &str = "target/debug/clippy-driver";
#[cfg(not(windows))]
const CARGO_CLIPPY_PATH: &str = "target/debug/cargo-clippy";

#[cfg(windows)]
const CLIPPY_DRIVER_PATH: &str = "target/debug/clippy-driver.exe";
#[cfg(windows)]
const CARGO_CLIPPY_PATH: &str = "target/debug/cargo-clippy.exe";

const VENDOR: &str = "target/lintcheck/vendor";
const PATCHES: &str = "target/lintcheck/patches.toml";

#[derive(Debug)]
struct ClippyWarning {
    crate_name: String,
    file: String,
    line: String,
    column: String,
    linttype: String,
    message: String,
    is_ice: bool,
}

#[allow(unused)]
impl ClippyWarning {
    fn to_output(&self, markdown: bool) -> String {
        let file_with_pos = format!("{}:{}:{}", self.file, self.line, self.column);
        if markdown {
            let lint = format!("`{}`", self.linttype);

            let mut output = String::from("| ");
            write!(output, "[`{}`]({}#L{})", file_with_pos, self.file, self.line).unwrap();
            write!(output, r#" | {:<50} | "{}" |"#, lint, self.message).unwrap();
            output
        } else {
            format!("{} {} \"{}\"", file_with_pos, self.linttype, self.message)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LintcheckConfig {
    /// we read the sources to check from here
    sources_toml_path: PathBuf,
    /// we save the clippy lint results here
    lintcheck_results_path: PathBuf,
    /// A list of lints that this lintcheck run should focus on
    lint_filter: Vec<String>,
    /// The list of packages to lint
    package_filter: Vec<String>,
    /// Indicate if the output should support markdown syntax
    markdown: bool,
    /// Enable cargo --timings
    timings: bool,
}

impl LintcheckConfig {
    fn from_clap(clap_config: &ArgMatches) -> Self {
        // first, check if we got anything passed via the LINTCHECK_TOML env var,
        // if not, ask clap if we got any value for --crates-toml  <foo>
        // if not, use the default "lintcheck/lintcheck_crates/Cargo.toml"
        let sources_toml = env::var("LINTCHECK_TOML").unwrap_or_else(|_| {
            clap_config
                .value_of("crates-toml")
                .clone()
                .unwrap_or("lintcheck/lintcheck_crates/Cargo.toml")
                .to_string()
        });

        let markdown = clap_config.is_present("markdown");
        let sources_toml_path = absolute(sources_toml).unwrap();

        // for the path where we save the lint results, use the directory of the sources Cargo.toml
        let filename = sources_toml_path.parent().unwrap().file_name().unwrap();
        let lintcheck_results_path = PathBuf::from(format!(
            "lintcheck-logs/{}_logs.{}",
            filename.to_str().unwrap(),
            if markdown { "md" } else { "txt" }
        ));

        let lint_filter: Vec<String> = clap_config
            .values_of("filter")
            .map(|iter| {
                iter.map(|lint_name| {
                    let mut filter = lint_name.replace('_', "-");
                    if !filter.starts_with("clippy::") {
                        filter.insert_str(0, "clippy::");
                    }
                    filter
                })
                .collect()
            })
            .unwrap_or_default();

        let package_filter = clap_config
            .values_of("package")
            .map(|iter| iter.map(|s| s.to_string()).collect())
            .unwrap_or_default();

        LintcheckConfig {
            sources_toml_path,
            lintcheck_results_path,
            lint_filter,
            package_filter,
            markdown,
            timings: clap_config.is_present("timings"),
        }
    }
}

/// takes a single json-formatted clippy warnings and returns true (we are interested in that line)
/// or false (we aren't)
fn filter_clippy_warnings(line: &str) -> bool {
    // we want to collect ICEs because clippy might have crashed.
    // these are summarized later
    if line.contains("internal compiler error: ") {
        return true;
    }
    // in general, we want all clippy warnings
    // however due to some kind of bug, sometimes there are absolute paths
    // to libcore files inside the message
    // or we end up with cargo-metadata output (https://github.com/rust-lang/rust-clippy/issues/6508)

    // filter out these message to avoid unnecessary noise in the logs
    if line.contains("clippy::")
        && !(line.contains("could not read cargo metadata")
            || (line.contains(".rustup") && line.contains("toolchains")))
    {
        return true;
    }
    false
}

/// Builds clippy inside the repo to make sure we have a clippy executable we can use.
fn build_clippy() {
    let status = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build clippy!");
    if !status.success() {
        eprintln!("Error: Failed to compile Clippy!");
        std::process::exit(1);
    }
}

/// Parse the json output of clippy and return a `ClippyWarning`
fn parse_json_message(json_message: &str) -> ClippyWarning {
    let jmsg: Value = serde_json::from_str(&json_message).unwrap_or_else(|e| panic!("Failed to parse json:\n{:?}", e));

    let file = jmsg["message"]["spans"][0]["file_name"].as_str().unwrap();
    let file = match Path::new(file).strip_prefix(env::current_dir().unwrap()) {
        Ok(stripped) => stripped.display().to_string(),
        Err(_) => file.to_string(),
    };

    let crate_name = jmsg["package_id"]
        .as_str()
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_string();

    ClippyWarning {
        crate_name,
        file,
        line: jmsg["message"]["spans"][0]["line_start"]
            .to_string()
            .trim_matches('"')
            .into(),
        column: jmsg["message"]["spans"][0]["text"][0]["highlight_start"]
            .to_string()
            .trim_matches('"')
            .into(),
        linttype: jmsg["message"]["code"]["code"].to_string().trim_matches('"').into(),
        message: jmsg["message"]["message"].to_string().trim_matches('"').into(),
        is_ice: json_message.contains("internal compiler error: "),
    }
}

/// Generate a short list of occurring lints-types and their count
fn gather_stats(clippy_warnings: &[ClippyWarning]) -> (String, HashMap<&String, usize>) {
    // count lint type occurrences
    let mut counter: HashMap<&String, usize> = HashMap::new();
    clippy_warnings
        .iter()
        .for_each(|wrn| *counter.entry(&wrn.linttype).or_insert(0) += 1);

    // collect into a tupled list for sorting
    let mut stats: Vec<(&&String, &usize)> = counter.iter().map(|(lint, count)| (lint, count)).collect();
    stats.sort_by_key(|(&lint, &count)| (cmp::Reverse(count), lint));

    let mut header = String::from("| lint                                               | count |\n");
    header.push_str("| -------------------------------------------------- | ----- |\n");
    let stats_string = stats
        .iter()
        .map(|(lint, count)| format!("| {:<50} |  {:>4} |\n", lint, count))
        .fold(header, |mut table, line| {
            table.push_str(&line);
            table
        });

    (stats_string, counter)
}

/// returns true if `path` is newer than `compare_to`, or either of the paths do not exist
fn newer_than(path: impl AsRef<Path>, compare_to: impl AsRef<Path>) -> bool {
    let mtime = match path.as_ref().metadata() {
        Ok(metadata) => metadata.modified().expect("failed to get metadata of file"),
        Err(e) => {
            assert_eq!(e.kind(), ErrorKind::NotFound, "failed to get modification date");
            return true;
        },
    };

    let compare_mtime = match compare_to.as_ref().metadata() {
        Ok(metadata) => metadata.modified().expect("failed to get metadata of file"),
        Err(e) => {
            assert_eq!(e.kind(), ErrorKind::NotFound, "failed to get modification date");
            return true;
        },
    };

    mtime > compare_mtime
}

/// check if the latest modification of the logfile is older than the modification date of the
/// clippy binary, if this is true, we should clean the lintcheck shared target directory and
/// recheck
fn lintcheck_needs_rerun(lintcheck_logs_path: &Path) -> bool {
    newer_than(CLIPPY_DRIVER_PATH, lintcheck_logs_path) || newer_than(CARGO_CLIPPY_PATH, lintcheck_logs_path)
}

/// check if the sources Cargo.toml has been modified since the last vendor run
fn vendor_needs_rerun(new_config: &LintcheckConfig) -> bool {
    let Ok(conf_bytes) = fs::read(PATCHES) else { return true };

    let conf: toml::Value = toml::from_slice(&conf_bytes).unwrap();
    let old_config = conf
        .get("lintcheck")
        .and_then(|v| v.get("config"))
        .map(|v| v.clone().try_into::<LintcheckConfig>());

    if let Some(Ok(old_config)) = old_config {
        old_config.sources_toml_path != new_config.sources_toml_path
            || newer_than(&new_config.sources_toml_path, VENDOR)
    } else {
        true
    }
}

fn read_ignores(config: &LintcheckConfig) -> Option<RegexSet> {
    let crates_toml = fs::read(&config.sources_toml_path).unwrap();
    let crates_toml = toml::from_slice::<toml::Value>(&crates_toml).unwrap();
    let ignore = crates_toml
        .get("package")?
        .get("metadata")?
        .get("lintcheck")?
        .get("ignore")?
        .as_array()
        .expect("expected ignore to be an array");

    let globs = ignore.iter().map(|v| {
        v.as_str()
            .expect("expected ignore entry to be a string")
            .replace('*', ".*")
    });
    Some(RegexSet::new(globs).unwrap())
}

fn create_patches(config: &LintcheckConfig) -> io::Result<()> {
    #[derive(Serialize, PartialOrd, PartialEq, Debug)]
    struct Patch {
        path: PathBuf,
        package: String,
    }

    let version_split = Regex::new(r"-\d+\.").unwrap();
    let ignore = read_ignores(config).unwrap_or_else(RegexSet::empty);
    let mut patches = BTreeMap::new();

    let vendor_absolute = absolute(VENDOR)?;
    for entry in fs::read_dir(VENDOR)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name().into_string().unwrap();
        let Some(package) = version_split.split(&name).next() else { continue };

        if ignore.is_match(package) {
            continue;
        }

        let patch = Patch {
            path: vendor_absolute.join(&name),
            package: package.to_string(),
        };

        patches.insert(name, patch);
    }

    let conf = toml::toml! {
        [source.crates-io]
        replace-with = "vendored"

        [source.vendored]
        directory = (toml::Value::try_from(vendor_absolute).unwrap())

        [patch]
        crates-io = (toml::Value::try_from(patches).unwrap())

        [lintcheck]
        config = (toml::Value::try_from(config).unwrap())
    };

    fs::write(PATCHES, conf.to_string())
}

fn clippy(config: &LintcheckConfig) -> Vec<u8> {
    let features = if config.package_filter.is_empty() {
        "--all-features".to_string()
    } else {
        format!("--features={}", config.package_filter.join(","))
    };

    let mut rustflags = vec!["--cap-lints=warn"];

    let lint_filter: Vec<String> = config.lint_filter.iter().map(|lint| format!("-W{lint}")).collect();
    if config.lint_filter.is_empty() {
        rustflags.extend(["-Wclippy::pedantic", "-Wclippy::cargo"]);
    } else {
        rustflags.extend(lint_filter.iter().map(|filter| filter.as_str()))
    }

    // We don't use `cargo clippy` as it uses RUSTC_WORKSPACE_WRAPPER=clippy-driver which doesn't
    // apply to patched packages. Instead we set RUSTC_WRAPPER=clippy-driver which applies to all
    // packages, but clippy-driver falls back to rustc when cargo passes `--cap-lints=allow` for
    // each unpatched package
    let output = Command::new("cargo")
        .args([
            "check",
            "-Zunstable-options",
            "-Zconfig-include",
            &features,
            "--message-format=json",
            "--target-dir=target/lintcheck/shared_target_dir",
            // cargo refreshes the progress bar when writing to stdout, which can cause flickering/slowness
            "--config=term.progress.when = \"never\"",
        ])
        .args(config.timings.then(|| "--timings"))
        .args(["--config", PATCHES])
        .args(["--manifest-path".as_ref(), config.sources_toml_path.as_os_str()])
        .env("RUSTFLAGS", rustflags.join(" "))
        .env("RUSTC_WRAPPER", absolute(CLIPPY_DRIVER_PATH).unwrap())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    if !output.status.success() {
        let filename = config.sources_toml_path.parent().unwrap().file_name().unwrap();
        let out = format!("lintcheck-logs/{}.stdout", filename.to_str().unwrap());

        fs::write(&out, output.stdout).unwrap();
        panic!("clippy returned {:?}, saved stdout to {}", output.status, out);
    }

    output.stdout
}

/// lintchecks `main()` function
///
/// # Panics
///
/// or if lintcheck is executed from the wrong directory (aka none-repo-root)
pub fn main() {
    // assert that we launch lintcheck from the repo root (via cargo lintcheck)
    if std::fs::metadata("lintcheck/Cargo.toml").is_err() {
        eprintln!("lintcheck needs to be run from clippys repo root!\nUse `cargo lintcheck` alternatively.");
        std::process::exit(3);
    }

    let clap_config = &get_clap_config();

    let config = LintcheckConfig::from_clap(clap_config);

    println!("Compiling clippy...");
    build_clippy();
    println!("Done compiling");

    // if the clippy bin is newer than our logs, throw away target dirs to force clippy to
    // refresh the logs
    if lintcheck_needs_rerun(&config.lintcheck_results_path) {
        let shared_target_dir = "target/lintcheck/shared_target_dir";
        // if we get an Err here, the shared target dir probably does simply not exist
        if let Ok(metadata) = std::fs::metadata(&shared_target_dir) {
            if metadata.is_dir() {
                println!("Clippy is newer than lint check logs, clearing lintcheck shared target dir...");
                std::fs::remove_dir_all(&shared_target_dir)
                    .expect("failed to remove target/lintcheck/shared_target_dir");
            }
        }
    }

    if vendor_needs_rerun(&config) {
        println!("Vendoring...");
        let status = Command::new("cargo")
            .args(["vendor", "--versioned-dirs"])
            .args(["--manifest-path".as_ref(), config.sources_toml_path.as_os_str()])
            .arg(VENDOR)
            // hide `[sources.crates-io] ...` output
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        println!("Vendored.")
    }

    create_patches(&config).unwrap();

    let stdout = clippy(&config);

    let clippy_ver = std::process::Command::new(CARGO_CLIPPY_PATH)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .expect("could not get clippy version!");

    let old_stats = read_stats_from_file(&config.lintcheck_results_path);

    let clippy_warnings: Vec<ClippyWarning> = String::from_utf8_lossy(&stdout)
        .lines()
        // get all clippy warnings and ICEs
        .filter(|line| filter_clippy_warnings(&line))
        .map(|json_msg| parse_json_message(json_msg))
        .collect();

    // generate some stats
    let (stats_formatted, new_stats) = gather_stats(&clippy_warnings);

    // grab crashes/ICEs, save the crate name and the ice message
    let ices: Vec<(&String, &String)> = clippy_warnings
        .iter()
        .filter(|warning| warning.is_ice)
        .map(|w| (&w.crate_name, &w.message))
        .collect();

    let mut all_msgs: Vec<String> = clippy_warnings
        .iter()
        .map(|warn| warn.to_output(config.markdown))
        .collect();
    all_msgs.sort();

    // save the text into lintcheck-logs/logs.txt
    let mut text = clippy_ver; // clippy version number on top
    text.push_str("\n\n### Stats:\n\n");
    text.push_str(&stats_formatted);
    text.push_str("\n### Reports\n\n");
    if config.markdown {
        text.push_str("| file | lint | message |\n");
        text.push_str("| --- | --- | --- |\n");
    }
    write!(text, "{}", all_msgs.join("\n")).unwrap();
    text.push_str("\n\n### ICEs:\n");
    for (cratename, msg) in ices.iter() {
        write!(text, "{}: '{}'", cratename, msg).unwrap();
    }

    println!("Writing logs to {}", config.lintcheck_results_path.display());
    fs::create_dir_all(config.lintcheck_results_path.parent().unwrap()).unwrap();
    fs::write(&config.lintcheck_results_path, text).unwrap();

    print_stats(old_stats, new_stats, &config.lint_filter);
}

/// read the previous stats from the lintcheck-log file
fn read_stats_from_file(file_path: &Path) -> HashMap<String, usize> {
    let file_content: String = match fs::read_to_string(file_path).ok() {
        Some(content) => content,
        None => {
            return HashMap::new();
        },
    };

    let lines: Vec<String> = file_content.lines().map(ToString::to_string).collect();

    lines
        .iter()
        .skip_while(|line| line.as_str() != "### Stats:")
        // Skipping the table header and the `Stats:` label
        .skip(4)
        .take_while(|line| line.starts_with("| "))
        .filter_map(|line| {
            let mut spl = line.split('|');
            // Skip the first `|` symbol
            spl.next();
            if let (Some(lint), Some(count)) = (spl.next(), spl.next()) {
                Some((lint.trim().to_string(), count.trim().parse::<usize>().unwrap()))
            } else {
                None
            }
        })
        .collect::<HashMap<String, usize>>()
}

/// print how lint counts changed between runs
fn print_stats(old_stats: HashMap<String, usize>, new_stats: HashMap<&String, usize>, lint_filter: &Vec<String>) {
    let same_in_both_hashmaps = old_stats
        .iter()
        .filter(|(old_key, old_val)| new_stats.get::<&String>(&old_key) == Some(old_val))
        .map(|(k, v)| (k.to_string(), *v))
        .collect::<Vec<(String, usize)>>();

    let mut old_stats_deduped = old_stats;
    let mut new_stats_deduped = new_stats;

    // remove duplicates from both hashmaps
    same_in_both_hashmaps.iter().for_each(|(k, v)| {
        assert!(old_stats_deduped.remove(k) == Some(*v));
        assert!(new_stats_deduped.remove(k) == Some(*v));
    });

    println!("\nStats:");

    // list all new counts  (key is in new stats but not in old stats)
    new_stats_deduped
        .iter()
        .filter(|(new_key, _)| old_stats_deduped.get::<str>(&new_key).is_none())
        .for_each(|(new_key, new_value)| {
            println!("{} 0 => {}", new_key, new_value);
        });

    // list all changed counts (key is in both maps but value differs)
    new_stats_deduped
        .iter()
        .filter(|(new_key, _new_val)| old_stats_deduped.get::<str>(&new_key).is_some())
        .for_each(|(new_key, new_val)| {
            let old_val = old_stats_deduped.get::<str>(&new_key).unwrap();
            println!("{} {} => {}", new_key, old_val, new_val);
        });

    // list all gone counts (key is in old status but not in new stats)
    old_stats_deduped
        .iter()
        .filter(|(old_key, _)| new_stats_deduped.get::<&String>(&old_key).is_none())
        .filter(|(old_key, _)| lint_filter.is_empty() || lint_filter.contains(old_key))
        .for_each(|(old_key, old_value)| {
            println!("{} {} => 0", old_key, old_value);
        });
}

fn get_clap_config<'a>() -> ArgMatches<'a> {
    App::new("lintcheck")
        .about("run clippy on a set of crates and check output")
        .arg(
            Arg::with_name("package")
                .takes_value(true)
                .value_name("PACKAGE")
                .multiple(true)
                .short("p")
                .long("package")
                .help("only lint specific top level package(s)"),
        )
        .arg(
            Arg::with_name("crates-toml")
                .takes_value(true)
                .value_name("CRATES-SOURCES-TOML-PATH")
                .long("crates-toml")
                .help("set the path for a Cargo.toml where lintcheck should read the sources from"),
        )
        .arg(
            Arg::with_name("filter")
                .long("--filter")
                .takes_value(true)
                .multiple(true)
                .value_name("clippy_lint_name")
                .help("apply a filter to only collect specified lints"),
        )
        .arg(
            Arg::with_name("markdown")
                .long("--markdown")
                .help("change the reports table to use markdown links"),
        )
        .arg(Arg::with_name("timings").long("--timings").help("enable cargo timings"))
        .get_matches()
}

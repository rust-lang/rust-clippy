#![cfg_attr(feature = "deny-warnings", deny(warnings))]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]

use rustc_tools_util::VersionInfo;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{self, Command};
use std::{collections::HashSet, env, path::Path};

mod lintlist;

const CARGO_CLIPPY_HELP: &str = r#"Checks a package to catch common mistakes and improve your Rust code.

Usage:
    cargo clippy [options] [--] [<opts>...]

Common options:
    -h, --help               Print this message
    -V, --version            Print version info and exit

Other options are the same as `cargo check`.

To allow or deny a lint from the command line you can use `cargo clippy --`
with:

    -W --warn OPT       Set lint warnings
    -A --allow OPT      Set lint allowed
    -D --deny OPT       Set lint denied
    -F --forbid OPT     Set lint forbidden

You can use tool lints to allow or deny lints from your code, eg.:

    #[allow(clippy::needless_lifetimes)]
"#;

fn show_help() {
    println!("{}", CARGO_CLIPPY_HELP);
}

fn show_version() {
    let version_info = rustc_tools_util::get_version_info!();
    println!("{}", version_info);
}

pub fn main() {
    // Check for version and help flags even when invoked as 'cargo-clippy'
    if env::args().any(|a| a == "--help" || a == "-h") {
        show_help();
        return;
    }

    if env::args().any(|a| a == "--version" || a == "-V") {
        show_version();
        return;
    }

    let mut orig_args: Vec<String> = env::args().collect();

    // Setting RUSTC_WRAPPER causes Cargo to pass 'rustc' as the first argument.
    // We're invoking the compiler programmatically, so we ignore this/
    let wrapper_mode = orig_args.get(1).map(Path::new).and_then(Path::file_stem) == Some("rustc".as_ref());

    if wrapper_mode {
        // we still want to be able to invoke it normally though
        orig_args.remove(1);
    }

    let should_describe_lints = || {
        let args: Vec<_> = env::args().collect();
        args.windows(2)
            .any(|args| args[1] == "help" && matches!(args[0].as_str(), "-W" | "-A" | "-D" | "-F"))
    };

    if !wrapper_mode && should_describe_lints() {
        describe_lints();
        return;
    }

    if let Err(code) = process(env::args().skip(2)) {
        process::exit(code);
    }
}

struct ClippyCmd {
    unstable_options: bool,
    cargo_subcommand: &'static str,
    args: Vec<String>,
    clippy_args: String,
}

impl ClippyCmd {
    fn new<I>(mut old_args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut cargo_subcommand = "check";
        let mut unstable_options = false;
        let mut args = vec![];

        for arg in old_args.by_ref() {
            match arg.as_str() {
                "--fix" => {
                    cargo_subcommand = "fix";
                    continue;
                },
                "--" => break,
                // Cover -Zunstable-options and -Z unstable-options
                s if s.ends_with("unstable-options") => unstable_options = true,
                _ => {},
            }

            args.push(arg);
        }

        if cargo_subcommand == "fix" && !unstable_options {
            panic!("Usage of `--fix` requires `-Z unstable-options`");
        }

        // Run the dogfood tests directly on nightly cargo. This is required due
        // to a bug in rustup.rs when running cargo on custom toolchains. See issue #3118.
        if env::var_os("CLIPPY_DOGFOOD").is_some() && cfg!(windows) {
            args.insert(0, "+nightly".to_string());
        }

        let clippy_args: String = old_args.map(|arg| format!("{}__CLIPPY_HACKERY__", arg)).collect();

        ClippyCmd {
            unstable_options,
            cargo_subcommand,
            args,
            clippy_args,
        }
    }

    fn path_env(&self) -> &'static str {
        if self.unstable_options {
            "RUSTC_WORKSPACE_WRAPPER"
        } else {
            "RUSTC_WRAPPER"
        }
    }

    fn path() -> PathBuf {
        let mut path = env::current_exe()
            .expect("current executable path invalid")
            .with_file_name("clippy-driver");

        if cfg!(windows) {
            path.set_extension("exe");
        }

        path
    }

    fn target_dir() -> Option<(&'static str, OsString)> {
        env::var_os("CLIPPY_DOGFOOD")
            .map(|_| {
                env::var_os("CARGO_MANIFEST_DIR").map_or_else(
                    || std::ffi::OsString::from("clippy_dogfood"),
                    |d| {
                        std::path::PathBuf::from(d)
                            .join("target")
                            .join("dogfood")
                            .into_os_string()
                    },
                )
            })
            .map(|p| ("CARGO_TARGET_DIR", p))
    }

    fn into_std_cmd(self) -> Command {
        let mut cmd = Command::new("cargo");

        cmd.env(self.path_env(), Self::path())
            .envs(ClippyCmd::target_dir())
            .env("CLIPPY_ARGS", self.clippy_args)
            .arg(self.cargo_subcommand)
            .args(&self.args);

        cmd
    }
}

fn process<I>(old_args: I) -> Result<(), i32>
where
    I: Iterator<Item = String>,
{
    let cmd = ClippyCmd::new(old_args);

    let mut cmd = cmd.into_std_cmd();

    let exit_status = cmd
        .spawn()
        .expect("could not run cargo")
        .wait()
        .expect("failed to wait for cargo?");

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap_or(-1))
    }
}

#[allow(clippy::find_map, clippy::filter_map)]
fn describe_lints() {
    use lintlist::{Level, Lint, ALL_LINTS, LINT_LEVELS};

    println!(
        "
Available lint options:
    -W <foo>           Warn about <foo>
    -A <foo>           Allow <foo>
    -D <foo>           Deny <foo>
    -F <foo>           Forbid <foo> (deny <foo> and all attempts to override)

"
    );

    let lint_level = |lint: &Lint| {
        LINT_LEVELS
            .iter()
            .find(|level_mapping| level_mapping.0 == lint.group)
            .map(|(_, level)| match level {
                Level::Allow => "allow",
                Level::Warn => "warn",
                Level::Deny => "deny",
            })
            .unwrap()
    };

    let mut lints: Vec<_> = ALL_LINTS.iter().collect();
    // The sort doesn't case-fold but it's doubtful we care.
    lints.sort_by_cached_key(|x: &&Lint| (lint_level(x), x.name));

    let max_lint_name_len = lints
        .iter()
        .map(|lint| lint.name.len())
        .map(|len| len + "clippy::".len())
        .max()
        .unwrap_or(0);

    let padded = |x: &str| {
        let mut s = " ".repeat(max_lint_name_len - x.chars().count());
        s.push_str(x);
        s
    };

    let scoped = |x: &str| format!("clippy::{}", x);

    let lint_groups: HashSet<_> = lints.iter().map(|lint| lint.group).collect();

    println!("Lint checks provided by clippy:\n");
    println!("    {}  {:7.7}  meaning", padded("name"), "default");
    println!("    {}  {:7.7}  -------", padded("----"), "-------");

    let print_lints = |lints: &[&Lint]| {
        for lint in lints {
            let name = lint.name.replace("_", "-");
            println!(
                "    {}  {:7.7}  {}",
                padded(&scoped(&name)),
                lint_level(lint),
                lint.desc
            );
        }
        println!("\n");
    };

    print_lints(&lints);

    let max_group_name_len = std::cmp::max(
        "clippy::all".len(),
        lint_groups
            .iter()
            .map(|group| group.len())
            .map(|len| len + "clippy::".len())
            .max()
            .unwrap_or(0),
    );

    let padded_group = |x: &str| {
        let mut s = " ".repeat(max_group_name_len - x.chars().count());
        s.push_str(x);
        s
    };

    println!("Lint groups provided by clippy:\n");
    println!("    {}  sub-lints", padded_group("name"));
    println!("    {}  ---------", padded_group("----"));
    println!("    {}  the set of all clippy lints", padded_group("clippy::all"));

    let print_lint_groups = || {
        for group in lint_groups {
            let name = group.to_lowercase().replace("_", "-");
            let desc = lints
                .iter()
                .filter(|&lint| lint.group == group)
                .map(|lint| lint.name)
                .map(|name| name.replace("_", "-"))
                .collect::<Vec<String>>()
                .join(", ");
            println!("    {}  {}", padded_group(&scoped(&name)), desc);
        }
        println!("\n");
    };

    print_lint_groups();
}

#[cfg(test)]
mod tests {
    use super::ClippyCmd;

    #[test]
    #[should_panic]
    fn fix_without_unstable() {
        let args = "cargo clippy --fix".split_whitespace().map(ToString::to_string);
        let _ = ClippyCmd::new(args);
    }

    #[test]
    fn fix_unstable() {
        let args = "cargo clippy --fix -Zunstable-options"
            .split_whitespace()
            .map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!("fix", cmd.cargo_subcommand);
        assert_eq!("RUSTC_WORKSPACE_WRAPPER", cmd.path_env());
        assert!(cmd.args.iter().any(|arg| arg.ends_with("unstable-options")));
    }

    #[test]
    fn check() {
        let args = "cargo clippy".split_whitespace().map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!("check", cmd.cargo_subcommand);
        assert_eq!("RUSTC_WRAPPER", cmd.path_env());
    }

    #[test]
    fn check_unstable() {
        let args = "cargo clippy -Zunstable-options"
            .split_whitespace()
            .map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!("check", cmd.cargo_subcommand);
        assert_eq!("RUSTC_WORKSPACE_WRAPPER", cmd.path_env());
    }
}

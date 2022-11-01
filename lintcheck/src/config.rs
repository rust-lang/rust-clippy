use clap::{Arg, ArgAction, ArgMatches, Command};
use std::env;
use std::path::PathBuf;

fn get_clap_config() -> ArgMatches {
    Command::new("lintcheck")
        .about("run clippy on a set of crates and check output")
        .args([
            Arg::new("only")
                .action(ArgAction::Set)
                .value_name("CRATE")
                .long("only")
                .help("Only process a single crate of the list"),
            Arg::new("crates-toml")
                .action(ArgAction::Set)
                .value_name("CRATES-SOURCES-TOML-PATH")
                .long("crates-toml")
                .help("Set the path for a crates.toml where lintcheck should read the sources from"),
            Arg::new("threads")
                .action(ArgAction::Set)
                .value_name("N")
                .value_parser(clap::value_parser!(usize))
                .short('j')
                .long("jobs")
                .help("Number of threads to use, 0 automatic choice"),
            Arg::new("fix")
                .long("fix")
                .help("Runs cargo clippy --fix and checks if all suggestions apply"),
            Arg::new("filter")
                .long("filter")
                .action(ArgAction::Append)
                .value_name("clippy_lint_name")
                .help("Apply a filter to only collect specified lints, this also overrides `allow` attributes"),
            Arg::new("markdown")
                .long("markdown")
                .help("Change the reports table to use markdown links"),
            Arg::new("json")
                .long("json")
                .help("Output the diagnostics as JSON")
                .conflicts_with("markdown"),
            Arg::new("recursive")
                .long("--recursive")
                .help("Run clippy on the dependencies of crates specified in crates-toml")
                .conflicts_with("threads")
                .conflicts_with("fix"),
        ])
        .subcommand(
            Command::new("diff")
                .about("Prints the difference between two `lintcheck --json` results")
                .args([Arg::new("old").required(true), Arg::new("new").required(true)]),
        )
        .get_matches()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Text,
    Markdown,
    Json,
}

impl OutputFormat {
    fn file_extension(self) -> &'static str {
        match self {
            OutputFormat::Text => "txt",
            OutputFormat::Markdown => "md",
            OutputFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LintcheckConfig {
    /// max number of jobs to spawn (default 1)
    pub max_jobs: usize,
    /// we read the sources to check from here
    pub sources_toml_path: PathBuf,
    /// we save the clippy lint results here
    pub lintcheck_results_path: PathBuf,
    /// Check only a specified package
    pub only: Option<String>,
    /// whether to just run --fix and not collect all the warnings
    pub fix: bool,
    /// A list of lints that this lintcheck run should focus on
    pub lint_filter: Vec<String>,
    /// The output format of the log file
    pub format: OutputFormat,
    /// Run clippy on the dependencies of crates
    pub recursive: bool,
    /// Diff the two `lintcheck --json` results
    pub diff: Option<(PathBuf, PathBuf)>,
}

impl LintcheckConfig {
    pub fn new() -> Self {
        let clap_config = get_clap_config();

        // first, check if we got anything passed via the LINTCHECK_TOML env var,
        // if not, ask clap if we got any value for --crates-toml  <foo>
        // if not, use the default "lintcheck/lintcheck_crates.toml"
        let sources_toml = env::var("LINTCHECK_TOML").unwrap_or_else(|_| {
            clap_config
                .get_one::<String>("crates-toml")
                .map_or("lintcheck/lintcheck_crates.toml", |s| &**s)
                .into()
        });

        let format = if clap_config.contains_id("markdown") {
            OutputFormat::Markdown
        } else if clap_config.contains_id("json") {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        };
        let sources_toml_path = PathBuf::from(sources_toml);

        // for the path where we save the lint results, get the filename without extension (so for
        // wasd.toml, use "wasd"...)
        let filename: PathBuf = sources_toml_path.file_stem().unwrap().into();
        let lintcheck_results_path = PathBuf::from(format!(
            "lintcheck-logs/{}_logs.{}",
            filename.display(),
            format.file_extension(),
        ));

        // look at the --threads arg, if 0 is passed, ask rayon rayon how many threads it would spawn and
        // use half of that for the physical core count
        // by default use a single thread
        let max_jobs = match clap_config.get_one::<usize>("threads") {
            Some(&0) => {
                // automatic choice
                // Rayon seems to return thread count so half that for core count
                rayon::current_num_threads() / 2
            },
            Some(&threads) => threads,
            // no -j passed, use a single thread
            None => 1,
        };

        let lint_filter: Vec<String> = clap_config
            .get_many::<String>("filter")
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

        let diff = clap_config.subcommand_matches("diff").map(|args| {
            let path = |arg| PathBuf::from(args.get_one::<String>(arg).unwrap());

            (path("old"), path("new"))
        });

        LintcheckConfig {
            max_jobs,
            sources_toml_path,
            lintcheck_results_path,
            only: clap_config.get_one::<String>("only").map(String::from),
            fix: clap_config.contains_id("fix"),
            lint_filter,
            format,
            recursive: clap_config.contains_id("recursive"),
            diff,
        }
    }
}

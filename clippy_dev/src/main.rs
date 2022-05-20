#![cfg_attr(feature = "deny-warnings", deny(warnings))]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use clippy_dev::{bless, fmt, lint, new_lint, serve, setup, update_lints};
use indoc::indoc;
fn main() {
    let matches = get_clap_config();

    match matches.subcommand() {
        ("bless", Some(matches)) => {
            bless::bless(matches.is_present("ignore-timestamp"));
        },
        ("fmt", Some(matches)) => {
            fmt::run(matches.is_present("check"), matches.is_present("verbose"));
        },
        ("update_lints", Some(matches)) => {
            if matches.is_present("print-only") {
                update_lints::print_lints();
            } else if matches.is_present("check") {
                update_lints::update(update_lints::UpdateMode::Check);
            } else {
                update_lints::update(update_lints::UpdateMode::Change);
            }
        },
        ("new_lint", Some(matches)) => {
            match new_lint::create(
                matches.value_of("pass"),
                matches.value_of("name"),
                matches.value_of("category"),
                matches.is_present("msrv"),
            ) {
                Ok(_) => update_lints::update(update_lints::UpdateMode::Change),
                Err(e) => eprintln!("Unable to create lint: {}", e),
            }
        },
        ("setup", Some(sub_command)) => match sub_command.subcommand() {
            ("intellij", Some(matches)) => {
                if matches.is_present("remove") {
                    setup::intellij::remove_rustc_src();
                } else {
                    setup::intellij::setup_rustc_src(
                        matches
                            .value_of("rustc-repo-path")
                            .expect("this field is mandatory and therefore always valid"),
                    );
                }
            },
            ("git-hook", Some(matches)) => {
                if matches.is_present("remove") {
                    setup::git_hook::remove_hook();
                } else {
                    setup::git_hook::install_hook(matches.is_present("force-override"));
                }
            },
            ("vscode-tasks", Some(matches)) => {
                if matches.is_present("remove") {
                    setup::vscode::remove_tasks();
                } else {
                    setup::vscode::install_tasks(matches.is_present("force-override"));
                }
            },
            _ => {},
        },
        ("remove", Some(sub_command)) => match sub_command.subcommand() {
            ("git-hook", Some(_)) => setup::git_hook::remove_hook(),
            ("intellij", Some(_)) => setup::intellij::remove_rustc_src(),
            ("vscode-tasks", Some(_)) => setup::vscode::remove_tasks(),
            _ => {},
        },
        ("serve", Some(matches)) => {
            let port = matches.value_of("port").unwrap().parse().unwrap();
            let lint = matches.value_of("lint");
            serve::run(port, lint);
        },
        ("lint", Some(matches)) => {
            let path = matches.value_of("path").unwrap();
            let args = matches.values_of("args").into_iter().flatten();
            lint::run(path, args);
        },
        ("rename_lint", Some(matches)) => {
            let old_name = matches.value_of("old_name").unwrap();
            let new_name = matches.value_of("new_name").unwrap_or(old_name);
            let uplift = matches.is_present("uplift");
            update_lints::rename(old_name, new_name, uplift);
        },
        _ => {},
    }
}

fn get_clap_config<'a>() -> ArgMatches<'a> {
    App::new("Clippy developer tooling")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("bless")
                .about("bless the test output changes")
                .arg(
                    Arg::with_name("ignore-timestamp")
                        .long("ignore-timestamp")
                        .help("Include files updated before clippy was built"),
                ),
        )
        .subcommand(
            SubCommand::with_name("fmt")
                .about("Run rustfmt on all projects and tests")
                .arg(
                    Arg::with_name("check")
                        .long("check")
                        .help("Use the rustfmt --check option"),
                )
                .arg(
                    Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("Echo commands run"),
                ),
        )
        .subcommand(
            SubCommand::with_name("update_lints")
                .about("Updates lint registration and information from the source code")
                .long_about(
                    "Makes sure that:\n \
                 * the lint count in README.md is correct\n \
                 * the changelog contains markdown link references at the bottom\n \
                 * all lint groups include the correct lints\n \
                 * lint modules in `clippy_lints/*` are visible in `src/lib.rs` via `pub mod`\n \
                 * all lints are registered in the lint store",
                )
                .arg(Arg::with_name("print-only").long("print-only").help(
                    "Print a table of lints to STDOUT. \
                 This does not include deprecated and internal lints. \
                 (Does not modify any files)",
                ))
                .arg(
                    Arg::with_name("check")
                        .long("check")
                        .help("Checks that `cargo dev update_lints` has been run. Used on CI."),
                ),
        )
        .subcommand(
            SubCommand::with_name("new_lint")
                .about("Create new lint and run `cargo dev update_lints`")
                .arg(
                    Arg::with_name("pass")
                        .short("p")
                        .long("pass")
                        .help("Specify whether the lint runs during the early or late pass")
                        .takes_value(true)
                        .possible_values(&["early", "late"])
                        .required(true),
                )
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("Name of the new lint in snake case, ex: fn_too_long")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("category")
                        .short("c")
                        .long("category")
                        .help("What category the lint belongs to")
                        .default_value("nursery")
                        .possible_values(&[
                            "style",
                            "correctness",
                            "suspicious",
                            "complexity",
                            "perf",
                            "pedantic",
                            "restriction",
                            "cargo",
                            "nursery",
                            "internal",
                            "internal_warn",
                        ])
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("msrv")
                        .long("msrv")
                        .help("Add MSRV config code to the lint"),
                ),
        )
        .subcommand(
            SubCommand::with_name("setup")
                .about("Support for setting up your personal development environment")
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("intellij")
                        .about("Alter dependencies so Intellij Rust can find rustc internals")
                        .arg(
                            Arg::with_name("remove")
                                .long("remove")
                                .help("Remove the dependencies added with 'cargo dev setup intellij'")
                                .required(false),
                        )
                        .arg(
                            Arg::with_name("rustc-repo-path")
                                .long("repo-path")
                                .short("r")
                                .help("The path to a rustc repo that will be used for setting the dependencies")
                                .takes_value(true)
                                .value_name("path")
                                .conflicts_with("remove")
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("git-hook")
                        .about("Add a pre-commit git hook that formats your code to make it look pretty")
                        .arg(
                            Arg::with_name("remove")
                                .long("remove")
                                .help("Remove the pre-commit hook added with 'cargo dev setup git-hook'")
                                .required(false),
                        )
                        .arg(
                            Arg::with_name("force-override")
                                .long("force-override")
                                .short("f")
                                .help("Forces the override of an existing git pre-commit hook")
                                .required(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("vscode-tasks")
                        .about("Add several tasks to vscode for formatting, validation and testing")
                        .arg(
                            Arg::with_name("remove")
                                .long("remove")
                                .help("Remove the tasks added with 'cargo dev setup vscode-tasks'")
                                .required(false),
                        )
                        .arg(
                            Arg::with_name("force-override")
                                .long("force-override")
                                .short("f")
                                .help("Forces the override of existing vscode tasks")
                                .required(false),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Support for undoing changes done by the setup command")
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(SubCommand::with_name("git-hook").about("Remove any existing pre-commit git hook"))
                .subcommand(SubCommand::with_name("vscode-tasks").about("Remove any existing vscode tasks"))
                .subcommand(
                    SubCommand::with_name("intellij")
                        .about("Removes rustc source paths added via `cargo dev setup intellij`"),
                ),
        )
        .subcommand(
            SubCommand::with_name("serve")
                .about("Launch a local 'ALL the Clippy Lints' website in a browser")
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .short("p")
                        .help("Local port for the http server")
                        .default_value("8000")
                        .validator_os(serve::validate_port),
                )
                .arg(Arg::with_name("lint").help("Which lint's page to load initially (optional)")),
        )
        .subcommand(
            SubCommand::with_name("lint")
                .about("Manually run clippy on a file or package")
                .after_help(indoc! {"
                    EXAMPLES
                        Lint a single file:
                            cargo dev lint tests/ui/attrs.rs

                        Lint a package directory:
                            cargo dev lint tests/ui-cargo/wildcard_dependencies/fail
                            cargo dev lint ~/my-project

                        Run rustfix:
                            cargo dev lint ~/my-project -- --fix

                        Set lint levels:
                            cargo dev lint file.rs -- -W clippy::pedantic
                            cargo dev lint ~/my-project -- -- -W clippy::pedantic
                "})
                .arg(
                    Arg::with_name("path")
                        .required(true)
                        .help("The path to a file or package directory to lint"),
                )
                .arg(
                    Arg::with_name("args")
                        .multiple(true)
                        .help("Pass extra arguments to cargo/clippy-driver"),
                ),
        )
        .subcommand(
            SubCommand::with_name("rename_lint")
                .about("Renames the given lint")
                .arg(
                    Arg::with_name("old_name")
                        .index(1)
                        .required(true)
                        .help("The name of the lint to rename"),
                )
                .arg(
                    Arg::with_name("new_name")
                        .index(2)
                        .required_unless("uplift")
                        .help("The new name of the lint"),
                )
                .arg(
                    Arg::with_name("uplift")
                        .long("uplift")
                        .help("This lint will be uplifted into rustc"),
                ),
        )
        .get_matches()
}

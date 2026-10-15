#![feature(rustc_private)]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]

use clap::{Args, Parser, Subcommand};
use clippy_dev::{
    ClippyInfo, UpdateMode, dogfood, edit_lints, fmt, lint, new_lint, new_parse_cx, release, serve, setup, sync,
};
use std::env;

fn main() {
    let dev = Dev::parse();
    let clippy = ClippyInfo::search_for_manifest();
    if let Err(e) = env::set_current_dir(&clippy.path) {
        panic!("error setting current directory to `{}`: {e}", clippy.path.display());
    }

    match dev.command {
        DevCommand::Bless => {
            eprintln!("use `cargo bless` to automatically replace `.stderr` and `.fixed` files as tests are being run");
        },
        DevCommand::Dogfood {
            fix,
            allow_dirty,
            allow_staged,
            allow_no_vcs,
        } => dogfood::dogfood(fix, allow_dirty, allow_staged, allow_no_vcs),
        DevCommand::Fmt { check } => fmt::run(UpdateMode::from_check(check)),
        DevCommand::UpdateLints { check } => new_parse_cx(|cx| {
            let data = cx.parse_lint_decls();
            cx.dcx.exit_on_err();
            data.gen_decls(UpdateMode::from_check(check));
        }),
        DevCommand::NewLint {
            pass,
            name,
            category,
            r#type,
            msrv,
        } => match new_lint::create(clippy.version, pass, &name, &category, r#type.as_deref(), msrv) {
            Ok(()) => new_parse_cx(|cx| {
                let data = cx.parse_lint_decls();
                cx.dcx.exit_on_err();
                data.gen_decls(UpdateMode::Change);
            }),
            Err(e) => eprintln!("Unable to create lint: {e}"),
        },
        DevCommand::Setup(SetupCommand { subcommand }) => match subcommand {
            SetupSubcommand::Intellij { remove, repo_path } => {
                if remove {
                    setup::intellij::remove_rustc_src();
                } else {
                    setup::intellij::setup_rustc_src(&repo_path);
                }
            },
            SetupSubcommand::GitHook { remove, force_override } => {
                if remove {
                    setup::git_hook::remove_hook();
                } else {
                    setup::git_hook::install_hook(force_override);
                }
            },
            SetupSubcommand::Toolchain {
                standalone,
                force,
                release,
                name,
            } => setup::toolchain::create(standalone, force, release, &name),
            SetupSubcommand::VscodeTasks { remove, force_override } => {
                if remove {
                    setup::vscode::remove_tasks();
                } else {
                    setup::vscode::install_tasks(force_override);
                }
            },
        },
        DevCommand::Remove(RemoveCommand { subcommand }) => match subcommand {
            RemoveSubcommand::Intellij => setup::intellij::remove_rustc_src(),
            RemoveSubcommand::GitHook => setup::git_hook::remove_hook(),
            RemoveSubcommand::VscodeTasks => setup::vscode::remove_tasks(),
        },
        DevCommand::Serve { port, lint } => serve::run(port, lint),
        DevCommand::Lint { path, edition, args } => lint::run(&path, &edition, args.iter()),
        DevCommand::RenameLint { old_name, new_name } => new_parse_cx(|cx| {
            edit_lints::rename(cx, clippy.version, &old_name, &new_name);
        }),
        DevCommand::Uplift { old_name, new_name } => new_parse_cx(|cx| {
            edit_lints::uplift(cx, clippy.version, &old_name, new_name.as_deref().unwrap_or(&old_name));
        }),
        DevCommand::Deprecate { name, reason } => {
            new_parse_cx(|cx| edit_lints::deprecate(cx, clippy.version, &name, &reason));
        },
        DevCommand::Sync(SyncCommand { subcommand }) => match subcommand {
            SyncSubcommand::UpdateNightly => sync::update_nightly(),
        },
        DevCommand::Release(ReleaseCommand { subcommand }) => match subcommand {
            ReleaseSubcommand::BumpVersion => release::bump_version(clippy.version),
        },
    }
}

fn lint_name(name: &str) -> Result<String, String> {
    let name = name.replace('-', "_");
    if let Some((pre, _)) = name.split_once("::") {
        Err(format!("lint name should not contain the `{pre}` prefix"))
    } else if name
        .bytes()
        .any(|x| !matches!(x, b'_' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'))
    {
        Err("lint name contains invalid characters".to_owned())
    } else {
        Ok(name)
    }
}

#[derive(Parser)]
#[command(name = "dev", about)]
struct Dev {
    #[command(subcommand)]
    command: DevCommand,
}

#[derive(Subcommand)]
enum DevCommand {
    /// Bless the test output changes
    Bless,
    /// Runs the dogfood test
    Dogfood {
        /// Apply the suggestions when possible
        #[arg(long)]
        fix: bool,
        /// Fix code even if the working directory has changes
        #[arg(long, requires = "fix")]
        allow_dirty: bool,
        /// Fix code even if the working directory has staged changes
        #[arg(long, requires = "fix")]
        allow_staged: bool,
        /// Fix code even if a VCS was not detected
        #[arg(long, requires = "fix")]
        allow_no_vcs: bool,
    },
    /// Run rustfmt on all projects and tests
    Fmt {
        /// Use the rustfmt --check option
        #[arg(long)]
        check: bool,
    },
    /// Updates lint registration and information from the source code
    ///
    /// Makes sure that: {n}
    /// * the lint count in README.md is correct {n}
    /// * the changelog contains markdown link references at the bottom {n}
    /// * all lint groups include the correct lints {n}
    /// * lint modules in `clippy_lints/*` are visible in `src/lib.rs` via `pub mod` {n}
    /// * all lints are registered in the lint store
    #[command(name = "update_lints")]
    UpdateLints {
        /// Checks that `cargo dev update_lints` has been run. Used on CI.
        #[arg(long)]
        check: bool,
    },
    /// Create a new lint and run `cargo dev update_lints`
    #[command(name = "new_lint")]
    NewLint {
        /// Specify whether the lint runs during the early or late pass
        #[arg(short, long, conflicts_with = "type", default_value = "late")]
        pass: new_lint::Pass,
        /// Name of the new lint in snake case, ex: `fn_too_long`
        #[arg(
            short,
            long,
            value_parser = lint_name,
        )]
        name: String,
        /// What category the lint belongs to
        #[arg(
            short,
            long,
            value_parser = [
                "style",
                "correctness",
                "suspicious",
                "complexity",
                "perf",
                "pedantic",
                "restriction",
                "cargo",
                "nursery",
            ],
            default_value = "nursery",
        )]
        category: String,
        /// What directory the lint belongs in
        #[arg(long)]
        r#type: Option<String>,
        /// Add MSRV config code to the lint
        #[arg(long)]
        msrv: bool,
    },
    /// Support for setting up your personal development environment
    Setup(SetupCommand),
    /// Support for removing changes done by the setup command
    Remove(RemoveCommand),
    /// Launch a local 'ALL the Clippy Lints' website in a browser
    Serve {
        /// Local port for the http server
        #[arg(short, long, default_value = "8000")]
        port: u16,
        /// Which lint's page to load initially (optional)
        #[arg(long)]
        lint: Option<String>,
    },
    #[expect(clippy::doc_markdown, clippy::doc_attr_ordering)]
    /// Manually run clippy on a file or package
    ///
    /// ## Examples
    ///
    /// Lint a single file: {n}
    ///     cargo dev lint tests/ui/attrs.rs
    ///
    /// Lint a package directory: {n}
    ///     cargo dev lint tests/ui-cargo/wildcard_dependencies/fail {n}
    ///     cargo dev lint ~/my-project
    ///
    /// Run rustfix: {n}
    ///     cargo dev lint ~/my-project -- --fix
    ///
    /// Set lint levels: {n}
    ///     cargo dev lint file.rs -- -W clippy::pedantic {n}
    ///     cargo dev lint ~/my-project -- -- -W clippy::pedantic
    Lint {
        /// The Rust edition to use
        #[arg(long, default_value = "2024")]
        edition: String,
        /// The path to a file or package directory to lint
        path: String,
        /// Pass extra arguments to cargo/clippy-driver
        args: Vec<String>,
    },
    /// Rename a lint
    #[command(name = "rename_lint")]
    RenameLint {
        /// The name of the lint to rename
        #[arg(value_parser = lint_name)]
        old_name: String,
        /// The new name of the lint
        #[arg(value_parser = lint_name)]
        new_name: String,
    },
    /// Deprecate the given lint
    Deprecate {
        /// The name of the lint to deprecate
        #[arg(value_parser = lint_name)]
        name: String,
        /// The reason for deprecation
        #[arg(long, short)]
        reason: String,
    },
    /// Sync between the rust repo and the Clippy repo
    Sync(SyncCommand),
    /// Manage Clippy releases
    Release(ReleaseCommand),
    /// Marks a lint as uplifted into rustc and removes its code
    Uplift {
        /// The name of the lint to uplift
        #[arg(value_parser = lint_name)]
        old_name: String,
        /// The name of the lint in rustc
        #[arg(value_parser = lint_name)]
        new_name: Option<String>,
    },
}

#[derive(Args)]
struct SetupCommand {
    #[command(subcommand)]
    subcommand: SetupSubcommand,
}

#[derive(Subcommand)]
enum SetupSubcommand {
    /// Alter dependencies so Intellij Rust can find rustc internals
    Intellij {
        /// Remove the dependencies added with 'cargo dev setup intellij'
        #[arg(long)]
        remove: bool,
        /// The path to a rustc repo that will be used for setting the dependencies
        #[arg(long, short, conflicts_with = "remove")]
        repo_path: String,
    },
    /// Add a pre-commit git hook that formats your code to make it look pretty
    GitHook {
        /// Remove the pre-commit hook added with 'cargo dev setup git-hook'
        #[arg(long)]
        remove: bool,
        /// Forces the override of an existing git pre-commit hook
        #[arg(long, short)]
        force_override: bool,
    },
    /// Install a rustup toolchain pointing to the local clippy build
    ///
    /// This creates a toolchain with symlinks pointing at
    /// `target/.../{clippy-driver,cargo-clippy}`, rebuilds of the project will be reflected in the
    /// created toolchain unless `--standalone` is passed
    Toolchain {
        /// Create a standalone toolchain by copying the clippy binaries instead
        /// of symlinking them
        ///
        /// Use this for example to create a toolchain, make a small change and then make another
        /// toolchain with a different name in order to easily compare the two
        #[arg(long, short)]
        standalone: bool,
        /// Override an existing toolchain
        #[arg(long, short)]
        force: bool,
        /// Point to --release clippy binary
        #[arg(long, short)]
        release: bool,
        /// Name of the toolchain
        #[arg(long, short, default_value = "clippy")]
        name: String,
    },
    /// Add several tasks to vscode for formatting, validation and testing
    VscodeTasks {
        /// Remove the tasks added with 'cargo dev setup vscode-tasks'
        #[arg(long)]
        remove: bool,
        /// Forces the override of existing vscode tasks
        #[arg(long, short)]
        force_override: bool,
    },
}

#[derive(Args)]
struct RemoveCommand {
    #[command(subcommand)]
    subcommand: RemoveSubcommand,
}

#[derive(Subcommand)]
enum RemoveSubcommand {
    /// Remove the dependencies added with 'cargo dev setup intellij'
    Intellij,
    /// Remove the pre-commit git hook
    GitHook,
    /// Remove the tasks added with 'cargo dev setup vscode-tasks'
    VscodeTasks,
}

#[derive(Args)]
struct SyncCommand {
    #[command(subcommand)]
    subcommand: SyncSubcommand,
}

#[derive(Subcommand)]
enum SyncSubcommand {
    /// Update nightly version in `rust-toolchain.toml` and `clippy_utils`
    #[command(name = "update_nightly")]
    UpdateNightly,
}

#[derive(Args)]
struct ReleaseCommand {
    #[command(subcommand)]
    subcommand: ReleaseSubcommand,
}

#[derive(Subcommand)]
enum ReleaseSubcommand {
    /// Bump the version in the Cargo.toml files
    #[command(name = "bump_version")]
    BumpVersion,
}

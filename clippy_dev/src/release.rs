use crate::sync::PUSH_PR_DESCRIPTION;
use crate::utils::{FileUpdater, UpdateStatus, Version, parse_cargo_package};

use std::fmt::{Display, Write};

use clap::ValueEnum;
use xshell::{Shell, cmd};

static CARGO_TOML_FILES: &[&str] = &[
    "clippy_config/Cargo.toml",
    "clippy_lints/Cargo.toml",
    "clippy_utils/Cargo.toml",
    "declare_clippy_lint/Cargo.toml",
    "Cargo.toml",
];

pub fn bump_version(mut version: Version) {
    version.minor += 1;

    let mut updater = FileUpdater::default();
    for file in CARGO_TOML_FILES {
        updater.update_file(file, &mut |_, src, dst| {
            let package = parse_cargo_package(src);
            if package.version_range.is_empty() {
                dst.push_str(src);
                UpdateStatus::Unchanged
            } else {
                dst.push_str(&src[..package.version_range.start]);
                write!(dst, "\"{}\"", version.toml_display()).unwrap();
                dst.push_str(&src[package.version_range.end..]);
                UpdateStatus::from_changed(src.get(package.version_range.clone()) != dst.get(package.version_range))
            }
        });
    }
}

#[derive(ValueEnum, Copy, Clone)]
pub enum Branch {
    Stable,
    Beta,
    Master,
}

impl Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Branch::Stable => write!(f, "stable"),
            Branch::Beta => write!(f, "beta"),
            Branch::Master => write!(f, "master"),
        }
    }
}

pub fn rustc_clippy_commit(rustc_path: String, branch: Branch) {
    let sh = Shell::new().expect("failed to create shell");
    sh.change_dir(rustc_path);

    let base = branch.to_string();
    cmd!(sh, "git fetch https://github.com/rust-lang/rust {base}")
        .run()
        .expect("failed to fetch base commit");
    let last_rustup_commit = cmd!(
        sh,
        "git log -1 --merges --grep=\"{PUSH_PR_DESCRIPTION}\" FETCH_HEAD -- src/tools/clippy"
    )
    .read()
    .expect("failed to run git log");

    let commit = last_rustup_commit
        .lines()
        .find(|c| c.contains("Sync from Clippy commit:"))
        .expect("no commit found")
        .trim()
        .rsplit_once('@')
        .expect("no commit hash found")
        .1;

    println!("{commit}");
}

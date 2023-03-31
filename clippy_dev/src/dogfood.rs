use crate::clippy_project_root;
use std::{
    io::{self, Write},
    process::Command,
};

/// # Panics
///
/// Panics if unable to run the dogfood test
pub fn dogfood(fix: bool, allow_dirty: bool, allow_staged: bool) {
    let mut cmd = Command::new("cargo");

    cmd.current_dir(clippy_project_root())
        .args(["test", "-r", "--test", "dogfood"])
        .args(["--features", "internal"])
        .args(["--", "dogfood_clippy"]);

    let mut dogfood_args = Vec::new();
    if fix {
        dogfood_args.push("--fix");
    }

    if allow_dirty {
        dogfood_args.push("--allow-dirty");
    }

    if allow_staged {
        dogfood_args.push("--allow-staged");
    }

    cmd.env("__CLIPPY_DOGFOOD_ARGS", dogfood_args.join(" "));

    let output = cmd.output().expect("failed to run command");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
}

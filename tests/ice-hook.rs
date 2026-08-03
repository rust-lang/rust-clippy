use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// This simulates clippy panicking due to ICE.
const FILE: &str = "\
#![allow(internal_features)]
#![feature(rustc_attrs)]

#[rustc_delayed_bug_from_inside_query]
fn main() {}
";

#[test]
fn ice_hook() {
    // This file is only created if the test ends up compiling.
    let mut dst = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    dst.push("ice-hook.rmeta");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clippy-driver"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("RUSTC_ICE", "0")
        .args([
            "--cap-lints=allow",
            "--crate-type=bin",
            "--emit=metadata",
            "--color=never",
            "-Z",
            "no-codegen",
            "-o",
        ])
        .arg(&dst)
        .arg("-")
        .spawn()
        .unwrap();
    cmd.stdin.take().unwrap().write_all(FILE.as_bytes()).unwrap();
    let output = cmd.wait_with_output().unwrap();

    let stderr = str::from_utf8(&output.stderr).unwrap();
    assert!(
        !output.status.success(),
        "test compilation succeeded, but it should have failed"
    );
    assert!(
        str::from_utf8(&output.stderr)
            .unwrap()
            .contains("error: internal compiler error: delayed bug"),
        "missing ICE message in output:\n\n{stderr}",
    );
}

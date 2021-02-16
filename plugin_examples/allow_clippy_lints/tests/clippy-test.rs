use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use std::fs::{canonicalize, metadata, read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

lazy_static! {
    static ref CLIPPY: PathBuf = canonicalize("../../target/debug/cargo-clippy").unwrap();
}

const PLUGIN: &str = "liballow_clippy_lints.so";

#[test]
fn clippy_test() {
    let src_base = Path::new("tests").join("ui");
    let plugin = Path::new("..")
        .join("..")
        .join("..")
        .join("target")
        .join("debug")
        .join(PLUGIN);

    for entry in read_dir(src_base).unwrap() {
        let path = entry.unwrap().path();

        if !metadata(&path).unwrap().is_dir() {
            continue;
        }

        Command::new(&*CLIPPY)
            .current_dir(&path)
            .args(&["cargo-clippy", "--quiet", "--plugin", &plugin.to_string_lossy()])
            .assert()
            .stdout(read_to_string(&*(path.to_string_lossy() + ".stdout")).unwrap())
            .stderr(read_to_string(&*(path.to_string_lossy() + ".stderr")).unwrap());
    }
}

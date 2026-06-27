use std::path::PathBuf;
use std::process::Command;
use test_utils::{CARGO_CLIPPY_PATH, IS_RUSTC_TEST_SUITE};

mod test_utils;

#[test]
fn test_allow_unwrap_in_integration_tests() {
    if IS_RUSTC_TEST_SUITE {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = root.join("target").join("integration_test");
    let cwd = root.join("tests/integration_test");

    // Make sure we start with a clean state
    Command::new("cargo")
        .current_dir(&cwd)
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("clean")
        .output()
        .unwrap();

    let output = Command::new(&*CARGO_CLIPPY_PATH)
        .current_dir(&cwd)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("clippy")
        .arg("--all-targets")
        .arg("--")
        .arg("-Cdebuginfo=0") // disable debuginfo to generate less data in the target dir
        .output()
        .unwrap();

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
}

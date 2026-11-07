use std::path::PathBuf;
use std::process::Command;
use test_utils::IS_RUSTC_TEST_SUITE;

mod test_utils;

#[test]
fn cargo_manifest_dir() {
    if IS_RUSTC_TEST_SUITE {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_clippy-driver"))
        .current_dir(&root)
        .env_remove("CLIPPY_CONF_DIR")
        .env("CARGO_MANIFEST_DIR", root.join("tests/ui-cargo/config_file_paths"))
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from))
        .arg(root.join("tests/ui-cargo/config_file_paths/src/main.rs"))
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("disallowed/placeholder name `selected`"), "{stderr}");
}

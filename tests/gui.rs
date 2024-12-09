// This test ensures that the clippy lints page is working as expected.

use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

fn get_available_browser_ui_test_version_inner(global: bool) -> Option<String> {
    let mut command = Command::new("npm");
    command.arg("list").arg("--parseable").arg("--long").arg("--depth=0");
    if global {
        command.arg("--global");
    }
    let stdout = command.output().expect("`npm` command not found").stdout;
    let lines = String::from_utf8_lossy(&stdout);
    lines
        .lines()
        .find_map(|l| l.split(':').nth(1)?.strip_prefix("browser-ui-test@"))
        .map(std::borrow::ToOwned::to_owned)
}

fn get_available_browser_ui_test_version() -> Option<String> {
    get_available_browser_ui_test_version_inner(false).or_else(|| get_available_browser_ui_test_version_inner(true))
}

fn expected_browser_ui_test_version() -> String {
    let content =
        read_to_string(".github/workflows/clippy.yml").expect("failed to read `.github/workflows/clippy.yml`");
    for line in content.lines() {
        let line = line.trim();
        if let Some(version) = line.strip_prefix("BROWSER_UI_TEST_VERSION:") {
            return version.trim().replace('\'', "");
        }
    }
    panic!("failed to retrieved `browser-ui-test` version");
}

fn mtime(path: impl AsRef<Path>) -> SystemTime {
    let path = path.as_ref();
    if path.is_dir() {
        path.read_dir()
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| mtime(entry.path()))
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH)
    } else {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

#[test]
fn check_clippy_lints_page() {
    // do not run this test inside the upstream rustc repo.
    if option_env!("RUSTC_TEST_SUITE").is_some() {
        return;
    }
    let browser_ui_test_version = expected_browser_ui_test_version();
    match get_available_browser_ui_test_version() {
        Some(version) => {
            if version != browser_ui_test_version {
                eprintln!(
                    "⚠️ Installed version of browser-ui-test (`{version}`) is different than the \
                     one used in the CI (`{browser_ui_test_version}`) You can install this version \
                     using `npm update browser-ui-test` or by using `npm install browser-ui-test\
                     @{browser_ui_test_version}`",
                );
            }
        },
        None => {
            panic!(
                "`browser-ui-test` is not installed. You can install this package using `npm \
                 update browser-ui-test` or by using `npm install browser-ui-test\
                 @{browser_ui_test_version}`",
            );
        },
    }

    // We build the lints page only if needed.
    let index_time = mtime("util/gh-pages/index.html");

    if (index_time < mtime("clippy_lints/src") || index_time < mtime("util/gh-pages/index_template.html"))
        && !Command::new("cargo")
            .arg("collect-metadata")
            .status()
            .is_ok_and(|status| status.success())
    {
        panic!("failed to run `cargo collect-metadata`");
    }

    let current_dir = std::env::current_dir()
        .expect("failed to retrieve current directory")
        .join("util/gh-pages/index.html");
    let current_dir = format!("file://{}", current_dir.display());
    let mut command = Command::new("npx");
    command
        .arg("browser-ui-test")
        .args(["--variable", "DOC_PATH", current_dir.as_str()])
        .args(["--test-folder", "tests/gui"]);
    if std::env::var_os("DISABLE_HEADLESS_TEST").is_some_and(|value| value == OsStr::new("1")) {
        command.arg("--no-headless");
    }

    // Then we run the GUI tests on it.
    assert!(command.status().is_ok_and(|status| status.success()));
}

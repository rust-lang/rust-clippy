//! The GUI test runner.
//!
//! This uses the browser-ui-test npm package to run a headless Chrome to
//! exercise the behavior of rendered books. See `CONTRIBUTING.md` for more
//! information.

use serde_json::Value;
use std::env::current_dir;
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;

fn get_available_browser_ui_test_version_inner(global: bool, test_dir: &str) -> Result<Option<String>, ()> {
    let mut command = Command::new("npm");
    command.current_dir(test_dir);
    command.arg("list").arg("--parseable").arg("--long").arg("--depth=0");
    if global {
        command.arg("--global");
    }
    let Ok(stdout) = command.output() else {
        eprintln!("`npm` command not found, ignoring GUI tests");
        return Err(());
    };
    let lines = String::from_utf8_lossy(&stdout.stdout);
    Ok(lines
        .lines()
        .find_map(|l| l.split(':').nth(1)?.strip_prefix("browser-ui-test@"))
        .map(std::borrow::ToOwned::to_owned))
}

fn get_available_browser_ui_test_version(test_dir: &str) -> Result<Option<String>, ()> {
    match get_available_browser_ui_test_version_inner(false, test_dir)? {
        Some(version) => Ok(Some(version)),
        None => get_available_browser_ui_test_version_inner(true, test_dir),
    }
}

fn expected_browser_ui_test_version(test_dir: &str) -> String {
    let content = read_to_string(Path::new(test_dir).join("package.json")).expect("failed to read `package.json`");
    let v: Value = serde_json::from_str(&content).expect("failed to parse `package.json`");
    let Some(dependencies) = v.get("dependencies") else {
        panic!("Missing `dependencies` key in `package.json`");
    };
    let Some(browser_ui_test) = dependencies.get("browser-ui-test") else {
        panic!("Missing `browser-ui-test` key in \"dependencies\" object in `package.json`");
    };
    let Value::String(version) = browser_ui_test else {
        panic!("`browser-ui-test` version was expected to be a string, but isn't");
    };
    version.trim().to_string()
}

fn main() {
    let test_dir = "tests/gui";
    let browser_ui_test_version = expected_browser_ui_test_version(test_dir);
    match get_available_browser_ui_test_version(test_dir) {
        Err(()) => return,
        Ok(Some(version)) => {
            if version != browser_ui_test_version {
                eprintln!(
                    "⚠️ Installed version of browser-ui-test (`{version}`) is different than the \
                     one used in the CI (`{browser_ui_test_version}`) You can install this version \
                     using `npm update browser-ui-test` or by using `npm install browser-ui-test\
                     @{browser_ui_test_version}` in the `{test_dir}` folder",
                );
            }
        },
        Ok(None) => {
            let msg = format_args!(
                "`browser-ui-test` is not installed. You can install this package using `npm \
                 update browser-ui-test` or by using `npm install browser-ui-test\
                 @{browser_ui_test_version}`"
            );

            if std::env::var("FORCE_GUI").is_ok_and(|v| v == "1") {
                panic!("{msg}");
            } else {
                println!(
                    "Ignoring `gui` test (can be overloaded with `FORCE_GUI=1` environment \
                     variable):"
                );
                println!("{msg}");
                return;
            }
        },
    }

    let current_dir = current_dir().expect("failed to retrieve current directory");
    let html_file = current_dir.join("util/gh-pages/index.html");
    assert!(
        html_file.is_file(),
        "Missing `{}`, run `cargo collect-metadata` first",
        html_file.display(),
    );
    let html_file = format!("file://{}", html_file.display());

    let mut command = Command::new("npx");
    command
        .arg("browser-ui-test")
        .args(["--variable", "DOC_PATH", html_file.as_str()])
        .args(["--display-format", "compact"])
        .current_dir(current_dir.join(test_dir));

    for arg in std::env::args().skip(1) {
        if arg.starts_with("--") {
            command.arg(arg);
        } else {
            command.args(["--filter", arg.as_str()]);
        }
    }

    command.args(["--test-folder", "."]);

    // Then we run the GUI tests on it.
    let status = command.status().expect("failed to get command status");
    assert!(status.success(), "{status:?}");
}

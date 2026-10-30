#![feature(rustc_private)]
use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

const ENABLED_GROUPS: [&str; 5] = ["all", "pedantic", "restriction", "nursery", "cargo"];

#[test]
fn doctest() {
    let clippy_driver = PathBuf::from(env!("CARGO_BIN_EXE_clippy-driver"));
    let mut command = Command::new("cargo");
    let command = command.args(["test", "--doc"]).current_dir("clippy_lints").env(
        "RUSTDOCFLAGS",
        format!(
            "--no-capture --no-run --test-builder {} -Zunstable-options {}",
            clippy_driver.display(),
            ENABLED_GROUPS
                .map(|group| format!("--doctest-build-arg=-Dclippy::{group}"))
                .join(" ")
        ),
    );

    let out = command.output().unwrap();

    let lints_declared = clippy_lints::declared_lints::LINTS
        .iter()
        .map(|lint| lint.name_lower())
        .collect::<Vec<_>>();

    let lints_mentioned = str::from_utf8(&out.stderr)
        .unwrap_or_else(|_| panic!("Could not get valid UTF-8 from test command: {command:#?}"))
        .lines()
        // Isolate the ones with the lint.
        .filter_map(|line| {
            if line.contains("for further information") {
                Some(line.split('#').nth(1).unwrap())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let lints_not_used = lints_declared
        .iter()
        .filter(|lint| !lints_mentioned.contains(&lint.as_str()))
        .collect::<Vec<_>>();

    if !lints_not_used.is_empty() {
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("tests/doctest.bless")
            .expect("Couldn't create tests/doctest.bless");
        if env::var_os("RUSTC_BLESS").is_some_and(|v| v != "0") {
            writeln!(
                &mut f,
                "// File generated automatically by doctest.rs, used to identify which lints do not (yet) have
// working doctests. This list MUST NOT get bigger. If the doctest.rs fails unexpectedly is probably
// because a new lint's documentation does not actually trigger said lint. You should review your
// documentation to make sure that the example actually triggers the lint.

"
            )
            .unwrap();
            writeln!(&mut f, "{lints_not_used:#?}").unwrap();
            return;
        }

        let mut blessed = String::new();
        f.read_to_string(&mut blessed).unwrap();

        blessed
            .lines()
            .filter(|line| line.ends_with(','))
            .zip(lints_not_used.clone())
            .for_each(|(blessed, actual_not_used)| {
                let blessed = blessed.split('"').nth(1).unwrap();
                assert!(
                    blessed == actual_not_used,
                    "Blessed doctests differ from actually failing ones: Expected {blessed}, found {actual_not_used}"
                );
            });
    }
}

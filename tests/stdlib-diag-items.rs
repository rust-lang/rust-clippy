// This tests checks that if a path is defined for an entity in the standard
// library, the proper prefix is used and a reference to a PR against
// `rust-lang/rust` is mentionned.

// This test is a no-op if run as part of the compiler test suite
// and will always succeed.

use itertools::Itertools;
use regex::Regex;
use std::io;

const PATHS_FILE: &str = "clippy_utils/src/paths.rs";

fn parse_content(content: &str) -> Vec<String> {
    let comment_re = Regex::new(r"^// `sym::(.*)` added in <https://github.com/rust-lang/rust/pull/\d+>$").unwrap();
    let path_re =
        Regex::new(r"^pub static ([A-Z_]+): PathLookup = (?:macro|type|value)_path!\((([a-z]+)::.*)\);").unwrap();
    let mut errors = vec![];
    for (prev, line) in content.lines().tuple_windows() {
        if let Some(caps) = path_re.captures(line) {
            if ["alloc", "core", "std"].contains(&&caps[3]) && !caps[1].starts_with("DIAG_ITEM_") {
                errors.push(format!(
                    "Path `{}` for `{}` should start with `DIAG_ITEM`",
                    &caps[1], &caps[2]
                ));
                continue;
            }
            if let Some(upper) = caps[1].strip_prefix("DIAG_ITEM_") {
                let Some(comment) = comment_re.captures(prev) else {
                    errors.push(format!(
                        "Definition for `{}` should be preceded by PR-related comment",
                        &caps[1]
                    ));
                    continue;
                };
                let upper_sym = comment[1].to_uppercase();
                if upper != upper_sym {
                    errors.push(format!(
                        "Path for symbol `{}` should be named `DIAG_ITEM_{}`",
                        &comment[1], upper_sym
                    ));
                }
            }
        }
    }
    errors
}

#[test]
fn stdlib_diag_items() -> Result<(), io::Error> {
    if option_env!("RUSTC_TEST_SUITE").is_some() {
        return Ok(());
    }

    let diagnostics = parse_content(&std::fs::read_to_string(PATHS_FILE)?);
    if diagnostics.is_empty() {
        Ok(())
    } else {
        eprintln!("Issues found in {PATHS_FILE}:");
        for diag in diagnostics {
            eprintln!("- {diag}");
        }
        Err(io::Error::other("problems found"))
    }
}

#[test]
fn internal_diag_items_test() {
    let content = r"
// Missing comment
pub static DIAG_ITEM_IO_ERROR_NEW: PathLookup = value_path!(std::io::Error::new);

// Wrong static name
// `sym::io_error` added in <https://github.com/rust-lang/rust/pull/142787>
pub static DIAG_ITEM_ERROR: PathLookup = value_path!(std::io::Error);

// Missing DIAG_ITEM
// `sym::io_foobar` added in <https://github.com/rust-lang/rust/pull/142787>
pub static IO_FOOBAR_PATH: PathLookup = value_path!(std::io);
";

    let diags = parse_content(content);
    let diags = diags.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(
        diags.as_slice(),
        [
            "Definition for `DIAG_ITEM_IO_ERROR_NEW` should be preceded by PR-related comment",
            "Path for symbol `io_error` should be named `DIAG_ITEM_IO_ERROR`",
            "Path `IO_FOOBAR_PATH` for `std::io` should start with `DIAG_ITEM`"
        ]
    );
}

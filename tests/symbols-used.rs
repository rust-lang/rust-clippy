// This test checks that all symbols defined in Clippy's `sym.rs` file
// are used in Clippy. Otherwise, it will fail with a list of symbols
// which are unused.
//
// This test is a no-op if run as part of the compiler test suite
// and will always succeed.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rayon::prelude::*;
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

const SYM_FILE: &str = "clippy_utils/src/sym.rs";

type Result<T, E = AnyError> = std::result::Result<T, E>;
type AnyError = Box<dyn std::error::Error>;

fn load_interned_symbols() -> Result<HashSet<String>> {
    let content = fs::read_to_string(SYM_FILE)?;
    let content = content
        .split_once("generate! {")
        .ok_or("cannot find symbols start")?
        .1
        .split_once("\n}\n")
        .ok_or("cannot find symbols end")?
        .0;
    Ok(Regex::new(r"(?m)^    (\w+)")
        .unwrap()
        .captures_iter(content)
        .map(|m| m[1].to_owned())
        .collect())
}

fn load_symbols(file: impl AsRef<Path>, re: &Regex) -> Result<Vec<String>> {
    Ok(re
        .captures_iter(&fs::read_to_string(file)?)
        .map(|m| m[1].to_owned())
        .collect())
}

fn load_paths(file: impl AsRef<Path>, re: &Regex) -> Result<Vec<String>> {
    Ok(re
        .captures_iter(&fs::read_to_string(file)?)
        .flat_map(|m| m[1].split("::").map(String::from).collect::<Vec<_>>())
        .collect())
}

#[test]
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn all_symbols_are_used() -> Result<()> {
    if option_env!("RUSTC_TEST_SUITE").is_some() {
        return Ok(());
    }

    let interned = load_interned_symbols()?;

    let used_re = Regex::new(r"\bsym::(\w+)\b").unwrap();
    let mut used = ["clippy_lints", "clippy_lints_internal", "clippy_utils", "src"]
        .par_iter()
        .flat_map(|dir| {
            WalkDir::new(dir)
                .into_iter()
                .filter_entry(|e| e.file_name().to_str().is_some_and(|s| s.ends_with(".rs")) || e.file_type().is_dir())
                .flat_map(|e| e.map(DirEntry::into_path))
                .par_bridge()
                // Silently ignore errors, this can never make this test pass while it should fail anyway
                .flat_map(|file| load_symbols(file, &used_re).unwrap_or_default())
        })
        .collect::<HashSet<_>>();

    let paths_re = Regex::new(r"path!\(([\w:]+)\)").unwrap();
    for path in [
        "clippy_utils/src/paths.rs",
        "clippy_lints_internal/src/internal_paths.rs",
    ] {
        used.extend(load_paths(path, &paths_re)?);
    }

    let mut extra = interned.difference(&used).collect::<Vec<_>>();
    if !extra.is_empty() {
        extra.sort_unstable();
        eprintln!("Unused symbols defined in {SYM_FILE}:");
        for sym in extra {
            eprintln!("  - {sym}");
        }
        Err(format!("extra symbols found — remove them {SYM_FILE}"))?;
    }
    Ok(())
}

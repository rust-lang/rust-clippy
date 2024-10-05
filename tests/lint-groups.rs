//! This test checks and updates `book/src/granular_lint_groups.md`
//!
//! Specifically the parts in between lint-group-start and lint-group-end are not meant to be edited
//! by hand and are instead generated based on the const `GROUPS` slice.
#![feature(rustc_private)]

use std::collections::HashSet;
use std::{env, fs};

use clippy_lints::LintInfo;
use clippy_lints::declared_lints::LINTS;
use indoc::formatdoc;
use itertools::Itertools;
use regex::{Captures, Regex};

const GROUPS: &[(&str, &[&str])] = &[
    ("perf-pedantic", &[
        "assigning_clones",
        "inefficient_to_string",
        "naive_bytecount",
        "needless_bitwise_bool",
        "trivially_copy_pass_by_ref",
        "unnecessary_join",
        "unnecessary_box_returns",
    ]),
    ("perf-restriction", &[
        "format_push_string",
        "missing_asserts_for_indexing",
    ]),
    ("perf-nursery", &[
        "redundant_clone",
        "iter_with_drain",
        "mutex_integer",
        "or_fun_call",
        "significant_drop_tightening",
        "trivial_regex",
        "needless_collect",
    ]),
    ("panicking", &[
        "arithmetic_side_effects",
        "expect_used",
        "unwrap_used",
        "panic",
        "unreachable",
        "todo",
        "unimplemented",
        "string_slice",
        "indexing_slicing",
    ]),
    ("debugging", &["dbg_macro", "todo", "unimplemented"]),
];

#[test]
fn check_lint_groups() {
    let file = fs::read_to_string("book/src/granular_lint_groups.md").expect("failed to read granular_lint_groups.md");
    let all_lint_names: HashSet<_> = LINTS
        .iter()
        .map(|LintInfo { lint, .. }| lint.name.strip_prefix("clippy::").unwrap().to_ascii_lowercase())
        .collect();

    let regex = Regex::new(
        "(?s)\
        (?<header><!-- lint-group-start: (?<name_start>[\\w-]+) -->)\
        (?<lints>.*?)\
        (?<footer><!-- lint-group-end: (?<name_end>[\\w-]+) -->)\
        ",
    )
    .unwrap();

    let replaced = regex.replace_all(&file, |captures: &Captures<'_>| -> String {
        let name = &captures["name_start"];

        assert_eq!(
            name, &captures["name_end"],
            "lint-group-start and lint-group-end lint names must match"
        );

        let lints = GROUPS
            .iter()
            .find_map(|&(name2, lints)| (name == name2).then_some(lints))
            .unwrap_or_else(|| panic!("lint group {name} does not exist"));

        for &lint in lints {
            assert!(
                all_lint_names.contains(lint),
                "lint {lint} in group {name} does not exist"
            );
        }

        let spoiler = |summary: &str, contents: &str| {
            formatdoc! {"
                <details>
                <summary>{summary}</summary>

                ```
                {contents}
                ```
                </details>
            "}
        };

        let lint_list = format!("Lints: {}", lints.iter().map(|lint| format!("`{lint}`")).join(", "));
        let warn_attr = spoiler(
            "#![warn] attribute",
            &format!(
                "#![warn({})]",
                lints.iter().map(|lint| format!("\n    clippy::{lint}")).join(",") + "\n"
            ),
        );
        let lint_table = spoiler(
            "Lint table",
            &format!(
                "[lints.clippy]\n{}",
                &lints.iter().map(|lint| format!(r#"{lint} = "warn""#)).join("\n")
            ),
        );

        format!(
            "{header}\n{lint_list}\n\n{warn_attr}\n{lint_table}{footer}",
            header = &captures["header"],
            footer = &captures["footer"]
        )
    });

    if replaced != file {
        if env::var_os("RUSTC_BLESS").is_some_and(|n| n != "0") {
            fs::write("book/src/granular_lint_groups.md", &*replaced).unwrap();
        } else {
            panic!("granular_lint_groups.md file has changed! Run `cargo bless --test lint-groups` to update.");
        }
    }
}

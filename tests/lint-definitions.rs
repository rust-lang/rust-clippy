#![feature(rustc_private)]

use std::collections::HashMap;
use std::fs;

use clippy_lints::declared_lints::LINTS;
use clippy_lints::deprecated_lints::RENAMED;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use test_utils::IS_RUSTC_TEST_SUITE;

mod test_utils;

#[test]
fn versions_match_changelog() {
    if IS_RUSTC_TEST_SUITE {
        return;
    }

    let changelog = fs::read_to_string("CHANGELOG.md").unwrap();

    let mut versions_by_name: HashMap<_, _> = LINTS.iter().map(|&lint| (lint.name_lower(), lint)).collect();

    for (from, to) in RENAMED {
        let from = from.strip_prefix("clippy::").unwrap();
        if let Some(to) = to.strip_prefix("clippy::") {
            versions_by_name.insert(from.to_owned(), versions_by_name[to]);
        }
    }

    let mut heading = None;
    let mut changelog_version = None;
    let mut in_new_lints = true;
    let mut checked = 0;

    for event in Parser::new(&changelog) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_new_lints = false;
                heading = Some(level);
            },
            Event::End(TagEnd::Heading(_)) => heading = None,
            Event::Text(text) => match heading {
                Some(HeadingLevel::H2) => {
                    if let Some(v) = text.strip_prefix("Rust ") {
                        changelog_version = Some(v.to_owned());
                    }
                },
                Some(HeadingLevel::H3) => {
                    in_new_lints = text.eq_ignore_ascii_case("new lints");
                },
                _ => {},
            },
            Event::Start(Tag::Link { id, .. }) if in_new_lints => {
                if let Some(name) = id.strip_prefix('`')
                    && let Some(name) = name.strip_suffix('`')
                    && let Some(&lint) = versions_by_name.get(name)
                {
                    let lint_version = lint.version.strip_suffix(".0").unwrap();
                    let changelog_version = changelog_version.as_deref().unwrap();
                    assert_eq!(
                        lint_version,
                        changelog_version,
                        "{name} has version {lint_version} but appears in the changelog for {changelog_version}\n\
                        \n\
                        update {} to `#[clippy::version = \"{changelog_version}.0\"]`",
                        lint.location_terminal(),
                    );
                    checked += 1;
                }
            },
            _ => {},
        }
    }

    assert!(
        checked > 400,
        "only checked {checked} versions, did the changelog format change?"
    );
}

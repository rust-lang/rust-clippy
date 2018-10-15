// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



#![allow(clippy::default_hash_types)]

use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::prelude::*;

lazy_static! {
    static ref DEC_CLIPPY_LINT_RE: Regex = Regex::new(r#"(?x)
        declare_clippy_lint!\s*[\{(]\s*
        pub\s+(?P<name>[A-Z_][A-Z_0-9]*)\s*,\s*
        (?P<cat>[a-z_]+)\s*,\s*
        "(?P<desc>(?:[^"\\]+|\\(?s).(?-s))*)"\s*[})]
    "#).unwrap();
    static ref DEC_DEPRECATED_LINT_RE: Regex = Regex::new(r#"(?x)
        declare_deprecated_lint!\s*[{(]\s*
        pub\s+(?P<name>[A-Z_][A-Z_0-9]*)\s*,\s*
        "(?P<desc>(?:[^"\\]+|\\(?s).(?-s))*)"\s*[})]
    "#).unwrap();
    static ref NL_ESCAPE_RE: Regex = Regex::new(r#"\\\n\s*"#).unwrap();
    pub static ref DOCS_LINK: String = "https://rust-lang-nursery.github.io/rust-clippy/master/index.html".to_string();
}

#[derive(Clone, PartialEq, Debug)]
pub struct Lint {
    pub name: String,
    pub group: String,
    pub desc: String,
    pub deprecation: Option<String>,
    pub module: String,
}

impl Lint {
    pub fn new(name: &str, group: &str, desc: &str, deprecation: Option<&str>, module: &str) -> Self {
        Self {
            name: name.to_lowercase(),
            group: group.to_string(),
            desc: NL_ESCAPE_RE.replace(&desc.replace("\\\"", "\""), "").to_string(),
            deprecation: deprecation.map(|d| d.to_string()),
            module: module.to_string(),
        }
    }

    /// Returns all non-deprecated lints
    pub fn active_lints(lints: impl Iterator<Item=Self>) -> impl Iterator<Item=Self> {
        lints.filter(|l| l.deprecation.is_none())
    }

    /// Returns the lints in a HashMap, grouped by the different lint groups
    pub fn by_lint_group(lints: &[Self]) -> HashMap<String, Vec<Self>> {
        lints.iter().map(|lint| (lint.group.to_string(), lint.clone())).into_group_map()
    }

    /// Generates `pub mod module_name` for the lints in the given `group`
    pub fn gen_pub_mod_for_group(lints: &[Lint]) -> Vec<String> {
        lints.into_iter().map(|l| format!("pub mod {};", l.module)).collect::<Vec<String>>()
    }

    pub fn gen_lint_group(lints: &[Lint]) -> Vec<String> {
        lints.into_iter().map(|l| format!("        {}::{},", l.module, l.name.to_uppercase())).collect::<Vec<String>>()
    }
}

pub fn gather_all() -> impl Iterator<Item=Lint> {
    lint_files().flat_map(|f| gather_from_file(&f))
}

fn gather_from_file(dir_entry: &fs::DirEntry) -> impl Iterator<Item=Lint> {
    let mut file = fs::File::open(dir_entry.path()).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    parse_contents(&content, dir_entry.path().file_stem().unwrap().to_str().unwrap())
}

fn parse_contents(content: &str, filename: &str) -> impl Iterator<Item=Lint> {
    let lints = DEC_CLIPPY_LINT_RE
        .captures_iter(content)
        .map(|m| Lint::new(&m["name"], &m["cat"], &m["desc"], None, filename));
    let deprecated = DEC_DEPRECATED_LINT_RE
        .captures_iter(content)
        .map(|m| Lint::new( &m["name"], "Deprecated", &m["desc"], Some(&m["desc"]), filename));
    // Removing the `.collect::<Vec<Lint>>().into_iter()` causes some lifetime issues due to the map
    lints.chain(deprecated).collect::<Vec<Lint>>().into_iter()
}

/// Collects all .rs files in the `clippy_lints/src` directory
fn lint_files() -> impl Iterator<Item=fs::DirEntry> {
    fs::read_dir("../clippy_lints/src")
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|f| f.path().extension() == Some(OsStr::new("rs")))
}


pub fn clippy_version_from_toml() -> String {
    let mut file = fs::File::open("../Cargo.toml").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let version_line = content.lines().find(|l| l.starts_with("version ="));
    if let Some(version_line) = version_line {
        let split = version_line.split(" ").collect::<Vec<&str>>();
        split[2].trim_matches('"').to_string()
    } else {
        panic!("Error: version not found in Cargo.toml!");
    }
}

pub fn replace_region_in_file<F>(path: &str, start: &str, end: &str, replace_start: bool, replacements: F) where F: Fn() -> Vec<String> {
    let mut f = fs::File::open(path).expect(&format!("File not found: {}", path));
    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("Something went wrong reading the file");
    let replaced = replace_region_in_text(&contents, start, end, replace_start, replacements);

    let mut f = fs::File::create(path).expect(&format!("File not found: {}", path));
    f.write_all(replaced.as_bytes()).expect("Unable to write file");
}

// Replace a region in a file delimited by two lines matching regexes.
//
// A callback is called to write the new region.
// If `replace_start` is true, the start delimiter line is replaced as well.
// The end delimiter line is never replaced.
pub fn replace_region_in_text<F>(text: &str, start: &str, end: &str, replace_start: bool, replacements: F) -> String where F: Fn() -> Vec<String> {
    let lines = text.lines();
    let mut in_old_region = false;
    let mut found = false;
    let mut new_lines = vec![];
    let start = Regex::new(start).unwrap();
    let end = Regex::new(end).unwrap();

    for line in lines {
        if in_old_region {
            if end.is_match(&line) {
                in_old_region = false;
                new_lines.extend(replacements());
                new_lines.push(line.to_string());
            }
        } else if start.is_match(&line) {
            if !replace_start {
                new_lines.push(line.to_string());
            }
            in_old_region = true;
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        println!("regex {:?} not found", start);
    }
    new_lines.join("\n")
}

#[test]
fn test_parse_contents() {
    let result: Vec<Lint> = parse_contents(
        r#"
declare_clippy_lint! {
    pub PTR_ARG,
    style,
    "really long \
     text"
}

declare_clippy_lint!{
    pub DOC_MARKDOWN,
    pedantic,
    "single line"
}

/// some doc comment
declare_deprecated_lint! {
    pub SHOULD_ASSERT_EQ,
    "`assert!()` will be more flexible with RFC 2011"
}
    "#,
    "module_name").collect();

    let expected = vec![
        Lint::new("ptr_arg", "style", "really long text", None, "module_name"),
        Lint::new("doc_markdown", "pedantic", "single line", None, "module_name"),
        Lint::new(
            "should_assert_eq",
            "Deprecated",
            "`assert!()` will be more flexible with RFC 2011",
            Some("`assert!()` will be more flexible with RFC 2011"),
            "module_name"
        ),
    ];
    assert_eq!(expected, result);
}


#[test]
fn test_replace_region() {
    let text = r#"
abc
123
789
def
ghi"#;
    let expected = r#"
abc
hello world
def
ghi"#;
    let result = replace_region_in_text(text, r#"^\s*abc$"#, r#"^\s*def"#, false, || {
        vec!["hello world".to_string()]
    });
    assert_eq!(expected, result);
}

#[test]
fn test_replace_region_with_start() {
    let text = r#"
abc
123
789
def
ghi"#;
    let expected = r#"
hello world
def
ghi"#;
    let result = replace_region_in_text(text, r#"^\s*abc$"#, r#"^\s*def"#, true, || {
        vec!["hello world".to_string()]
    });
    assert_eq!(expected, result);
}

#[test]
fn test_active_lints() {
    let lints = vec![
        Lint::new("should_assert_eq", "Deprecated", "abc", Some("Reason"), "module_name"),
        Lint::new("should_assert_eq2", "Not Deprecated", "abc", None, "module_name")
    ];
    let expected = vec![
        Lint::new("should_assert_eq2", "Not Deprecated", "abc", None, "module_name")
    ];
    assert_eq!(expected, Lint::active_lints(lints.into_iter()).collect::<Vec<Lint>>());
}

#[test]
fn test_by_lint_group() {
    let lints = vec![
        Lint::new("should_assert_eq", "group1", "abc", None, "module_name"),
        Lint::new("should_assert_eq2", "group2", "abc", None, "module_name"),
        Lint::new("incorrect_match", "group1", "abc", None, "module_name"),
    ];
    let mut expected: HashMap<String, Vec<Lint>> = HashMap::new();
    expected.insert("group1".to_string(), vec![
        Lint::new("should_assert_eq", "group1", "abc", None, "module_name"),
        Lint::new("incorrect_match", "group1", "abc", None, "module_name"),
    ]);
    expected.insert("group2".to_string(), vec![
        Lint::new("should_assert_eq2", "group2", "abc", None, "module_name")
    ]);
    assert_eq!(expected, Lint::by_lint_group(&lints));
}

#[test]
fn test_gen_pub_mod_for_group() {
    let lints = vec![
        Lint::new("should_assert_eq", "Deprecated", "abc", Some("Reason"), "abc"),
        Lint::new("should_assert_eq2", "Not Deprecated", "abc", None, "module_name"),
    ];
    let expected = vec![
        "pub mod abc;".to_string(),
        "pub mod module_name;".to_string(),
    ];
    assert_eq!(expected, Lint::gen_pub_mod_for_group(&lints));
}

#[test]
fn test_gen_lint_group() {
    let lints = vec![
        Lint::new("should_assert_eq", "Deprecated", "abc", Some("Reason"), "abc"),
        Lint::new("should_assert_eq2", "Not Deprecated", "abc", None, "module_name"),
    ];;
    let expected = vec![
        "        abc::SHOULD_ASSERT_EQ,".to_string(),
        "        module_name::SHOULD_ASSERT_EQ2,".to_string()
    ];
    assert_eq!(expected, Lint::gen_lint_group(&lints));
}

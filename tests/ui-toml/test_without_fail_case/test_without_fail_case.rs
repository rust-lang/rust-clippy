//@aux-build:test_macro.rs
//@compile-flags: --test
//@revisions: default fail_macro no_fail_macro index_fail
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/test_without_fail_case/default
//@[fail_macro] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/test_without_fail_case/fail_macro
//@[no_fail_macro] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/test_without_fail_case/no_fail_macro
//@[index_fail] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/test_without_fail_case/index_fail
#![allow(unused)]
#![allow(clippy::tests_outside_test_module)]
#![warn(clippy::test_without_fail_case)]
use test_macro::{fallible_macro, non_fallible_macro};

// Should not lint because of the clippy.toml file that adds `test` as fallible.
#[test]
fn test_custom_macro_fallible() {
    println!("a")
}

// Should not lint unless the clippy.toml file makes indexing fallible.
#[test]
fn test_indexing_fallible() {
    let a = [1, 2, 3];
    let _ = a[0];
}

#[test]
fn func_a() {
    let _ = 1;
}

#[test]
fn should_not_lint_if_defined_as_fallible() {
    non_fallible_macro!(1);
}

#[test]
fn should_lint_if_defined_as_non_fallible() {
    fallible_macro!(1);
}

fn main() {}

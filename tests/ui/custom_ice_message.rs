//@rustc-env:RUST_BACKTRACE=0
//@normalize-stderr-test: "Clippy version: .*" -> "Clippy version: foo"
//@normalize-stderr-test: "produce_ice.rs:\d*:\d*" -> "produce_ice.rs"
//@normalize-stderr-test: "', .*clippy_lints" -> "', clippy_lints"
//@normalize-stderr-test: "'rustc'" -> "'<unnamed>'"
//@normalize-stderr-test: "rustc 1\.\d+.* running on .*" -> "rustc <version> running on <target>"
//@normalize-stderr-test: "(?ms)query stack during panic:\n.*end of query stack\n" -> ""

#![feature(rustc_attrs)]

#[rustc_delayed_bug_from_inside_query]
fn main() {}
//~^ ice: delayed bug triggered by

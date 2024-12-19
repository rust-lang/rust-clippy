//@ normalize-stderr-test: "located at `.+/.crates.toml`" -> "located at `$$DIR/.crates.toml`"
//@ normalize-stderr-test: "folder \(`.+`" -> "folder (`$$CLIPPY_DIR`"

#![deny(clippy::include_file_outside_project)]

// Should not lint.
include!("./auxiliary/external_consts.rs");

fn main() {
    let x = include_str!(concat!(env!("CARGO_HOME"), "/.crates.toml"));
    //~^ include_file_outside_project
}

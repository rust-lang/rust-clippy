//@no-rustfix
//@aux-build:assert_is_empty_glob_helper.rs
#![warn(clippy::assert_is_empty)]
#![allow(clippy::const_is_empty)]

// `assert_is_empty_glob_helper` exports a module named `std` and a type named
// `i32`, so a glob import from it shadows the printed element types below. These
// suggestions are `MaybeIncorrect`. They do not compile.

extern crate assert_is_empty_glob_helper;

mod globbed {
    use assert_is_empty_glob_helper::*;

    pub fn glob_import() {
        let strings = [String::new()];
        assert!(strings.is_empty());
        //~^ assert_is_empty

        let numbers = [1i32];
        assert!(numbers.is_empty());
        //~^ assert_is_empty
    }
}

fn main() {
    globbed::glob_import();
}

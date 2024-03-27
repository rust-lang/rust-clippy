#![warn(clippy::module_name_repetitions)]
#![allow(dead_code)]

mod foo {
    // #12544 - shouldn't warn if the only part aside from the the module name is a preposition.
    // In this test, prepositions are configured to be ["bar"].

    // this line shouldn't produce a warning:
    pub fn to_foo() {}

    // but this line should
    pub fn bar_foo() {}
}

fn main() {}

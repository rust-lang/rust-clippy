#![warn(clippy::module_name_repetitions)]
#![allow(dead_code)]

mod foo {
    // #12544 - shouldn't warn if the only part aside from the the module name is a preposition.
    // In this test, prepositions are configured to be all of the default prepositions and ["bar"].

    // this line should produce a warning:
    pub fn something_foo() {}

    // but none of the following should:
    pub fn bar_foo() {}
    pub fn to_foo() {}
    pub fn as_foo() {}
    pub fn into_foo() {}
    pub fn from_foo() {}
}

fn main() {}

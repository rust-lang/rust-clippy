//@aux-build:../../ui/auxiliary/proc_macros.rs
//@revisions: default ord_within
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/arbitrary_source_item_ordering/default
//@[ord_within] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/arbitrary_source_item_ordering/ord_within

#![allow(dead_code)]
#![deny(clippy::arbitrary_source_item_ordering)]

#[allow(clippy::arbitrary_source_item_ordering)]
struct Ordered {
    a: bool,
    b: bool,
}

#[allow(clippy::arbitrary_source_item_ordering)]
struct Unordered {
    b: bool,
    a: bool,
}

#[deny(clippy::arbitrary_source_item_ordering)]
struct OrderedChecked {
    a: bool,
    b: bool,
}

#[deny(clippy::arbitrary_source_item_ordering)]
struct UnorderedChecked {
    b: bool,
    a: bool,
}

fn main() {
    // test code goes here
}

fn before_main() {}

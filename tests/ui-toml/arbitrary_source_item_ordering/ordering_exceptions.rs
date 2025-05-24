//@aux-build:../../ui/auxiliary/proc_macros.rs
//@revisions: exceptions
//@[exceptions] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/arbitrary_source_item_ordering/exceptions

#![allow(dead_code)]
#![warn(clippy::arbitrary_source_item_ordering)]

struct BasicStruct1 {}

impl BasicStruct1 {
    fn new() -> Self {
        Self {}
    }
    fn a() {}
    fn b() {}
}

struct BasicStruct2 {}

impl BasicStruct2 {
    fn a() {}
    fn new() -> Self {
        Self {}
    }
    fn b() {}
}

struct BasicStruct3 {}

impl BasicStruct3 {
    fn b() {}
    fn new() -> Self {
        Self {}
    }
    fn a() {}
    //~^ arbitrary_source_item_ordering
}

struct BasicStruct4 {}

impl BasicStruct4 {
    fn a() {}
    fn new() -> Self {
        Self {}
    }
    fn c() {}
    fn b() {}
    //~^ arbitrary_source_item_ordering
}

struct BasicStruct5 {}

impl BasicStruct5 {
    fn new() -> Self {
        Self {}
    }
    fn b() {}
    fn c() {}
    fn a() {}
    //~^ arbitrary_source_item_ordering
}

fn main() {
    // test code goes here
}

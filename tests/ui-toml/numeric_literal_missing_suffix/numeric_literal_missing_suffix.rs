//@run-rustfix
//@aux-build:../../ui/auxiliary/proc_macros.rs:proc-macro
//@revisions: disallow_type_annotations allow_type_annotations
//@[disallow_type_annotations] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/numeric_literal_missing_suffix/disallow_type_annotations
//@[allow_type_annotations] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/numeric_literal_missing_suffix/allow_type_annotations
#![allow(unused)]
#![warn(clippy::numeric_literal_missing_suffix)]
#![feature(custom_inner_attributes)]
#![rustfmt::skip] // Not sure why rustfmt messes up the uitest commands

#[macro_use]
extern crate proc_macros;

fn main() {
    let x = 1;
    let x = 1.0;
    let x: u8 = 1;
    static X1: u32 = 0;
    const X2: u32 = 0;
    // Don't lint
    external! {
        let x = 1;
        let x: u8 = 1;
        static X3: u32 = 0;
        const X4: u32 = 0;
    }
    with_span! {
        span
        let x = 1;
        let x: u8 = 1;
        static X5: u32 = 0;
        const X6: u32 = 0;
    }
}

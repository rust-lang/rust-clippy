//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::no_effect, deprecated, unused)]
#![warn(clippy::legacy_numeric_constants)]

#[macro_use]
extern crate proc_macros;

use std::u128 as _;

fn main() {
    use std::u32;
    u32::MAX;
    use std::f64;
    f64::MAX;
    u128::MAX;
    // Don't lint
    u8::MIN;
    std::f32::consts::E;
    use std::f32;
    f64::consts::E;
}

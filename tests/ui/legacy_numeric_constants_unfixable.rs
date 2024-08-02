//@no-rustfix
//@aux-build:proc_macros.rs
#![allow(clippy::no_effect, deprecated, unused)]
#![warn(clippy::legacy_numeric_constants)]

#[macro_use]
extern crate proc_macros;

use std::u128 as _;
//~^ legacy_numeric_constants
pub mod a {
    pub use std::{mem, u128};
    //~^ legacy_numeric_constants
}

macro_rules! b {
    () => {
        mod b {
            use std::u32;
        }
    };
}

fn main() {
    use std::u32::MAX;
    //~^ legacy_numeric_constants
    use std::u8::MIN;
    //~^ legacy_numeric_constants
    f64::MAX;
    use std::u32;
    //~^ legacy_numeric_constants
    u32::MAX;
    use std::f32::MIN_POSITIVE;
    //~^ legacy_numeric_constants
    use std::f64;
    use std::i16::*;
    //~^ legacy_numeric_constants
    u128::MAX;
    f32::EPSILON;
    f64::EPSILON;
    ::std::primitive::u8::MIN;
    std::f32::consts::E;
    f64::consts::E;
    u8::MIN;
    std::f32::consts::E;
    f64::consts::E;
    b!();
}

fn ext() {
    external! {
        ::std::primitive::u8::MIN;
        ::std::u8::MIN;
        ::std::primitive::u8::min_value();
        use std::u64;
        use std::u8::MIN;
    }
}

#[clippy::msrv = "1.42.0"]
fn msrv_too_low() {
    use std::u32::MAX;
}

#[clippy::msrv = "1.43.0"]
fn msrv_juust_right() {
    use std::u32::MAX;
    //~^ legacy_numeric_constants
}

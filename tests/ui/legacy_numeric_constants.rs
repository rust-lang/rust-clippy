//@aux-build:proc_macros.rs
#![allow(clippy::no_effect, deprecated, unused)]
#![allow(clippy::legacy_numeric_constants)] // For imports.

#[macro_use]
extern crate proc_macros;

pub mod a {
    pub use std::u128;
}

macro_rules! b {
    () => {
        mod b {
            #[warn(clippy::legacy_numeric_constants)]
            fn b() {
                let x = std::u64::MAX;
            }
        }
    };
}

use std::u32::MAX;
use std::u8::MIN;
use std::{f64, u32};

#[warn(clippy::legacy_numeric_constants)]
fn main() {
    std::f32::EPSILON;
    //~^ legacy_numeric_constants
    std::u8::MIN;
    //~^ legacy_numeric_constants
    std::usize::MIN;
    //~^ legacy_numeric_constants
    std::u32::MAX;
    //~^ legacy_numeric_constants
    core::u32::MAX;
    //~^ legacy_numeric_constants
    MAX;
    //~^ legacy_numeric_constants
    i32::max_value();
    //~^ legacy_numeric_constants
    u8::max_value();
    //~^ legacy_numeric_constants
    u8::min_value();
    //~^ legacy_numeric_constants
    ::std::u8::MIN;
    //~^ legacy_numeric_constants
    ::std::primitive::u8::min_value();
    //~^ legacy_numeric_constants
    std::primitive::i32::max_value();
    //~^ legacy_numeric_constants
    self::a::u128::MAX;
    //~^ legacy_numeric_constants
    u32::MAX;
    u128::MAX;
    f32::EPSILON;
    ::std::primitive::u8::MIN;
    std::f32::consts::E;
    f64::consts::E;
    u8::MIN;
    std::f32::consts::E;
    f64::consts::E;
    b!();

    [(0, "", std::i128::MAX)];
    //~^ legacy_numeric_constants
}

#[warn(clippy::legacy_numeric_constants)]
fn ext() {
    external! {
        ::std::primitive::u8::MIN;
        ::std::u8::MIN;
        ::std::primitive::u8::min_value();
        use std::u64;
        use std::u8::MIN;
    }
}

#[allow(clippy::legacy_numeric_constants)]
fn allow() {
    ::std::primitive::u8::MIN;
    ::std::u8::MIN;
    ::std::primitive::u8::min_value();
    use std::u64;
    use std::u8::MIN;
}

#[warn(clippy::legacy_numeric_constants)]
#[clippy::msrv = "1.42.0"]
fn msrv_too_low() {
    std::u32::MAX;
}

#[warn(clippy::legacy_numeric_constants)]
#[clippy::msrv = "1.43.0"]
fn msrv_juust_right() {
    std::u32::MAX;
    //~^ legacy_numeric_constants
}

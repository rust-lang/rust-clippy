//@aux-build:proc_macros.rs
#![allow(clippy::no_effect, deprecated, unused)]
#![warn(clippy::legacy_numeric_constants)]

#[macro_use]
extern crate proc_macros;

use std::u128 as _;
pub mod a {
    pub use std::u128;
}

fn main() {
    std::f32::EPSILON;
    std::u8::MIN;
    std::usize::MIN;
    std::u32::MAX;
    core::u32::MAX;
    use std::u32;
    use std::u32::MAX;
    MAX;
    u32::MAX;
    u32::max_value();
    u8::max_value();
    u8::min_value();
    ::std::primitive::u8::MIN;
    ::std::u8::MIN;
    ::std::primitive::u8::min_value();
    std::primitive::u32::max_value();
    use std::f64;
    f64::MAX;
    self::a::u128::MAX;
    u128::MAX;
    // Don't lint
    f32::EPSILON;
    u8::MIN;
    std::f32::consts::E;
    use std::f32;
    f64::consts::E;
    external! {
        ::std::primitive::u8::MIN;
        ::std::u8::MIN;
        ::std::primitive::u8::min_value();
    }
}

#[clippy::msrv = "1.42.0"]
fn msrv_too_low() {
    // FIXME: Why does this lint??
    std::u32::MAX;
}

#[clippy::msrv = "1.43.0"]
fn msrv_juust_right() {
    std::u32::MAX;
}

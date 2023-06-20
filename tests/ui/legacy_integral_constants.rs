//@run-rustfix
//@aux-build:proc_macros.rs
#![allow(clippy::no_effect, deprecated, unused)]
#![warn(clippy::legacy_integral_constants)]

#[macro_use]
extern crate proc_macros;

fn main() {
    std::f32::EPSILON;
    std::u8::MIN;
    std::usize::MIN;
    std::u32::MAX;
    use std::u32::MAX;
    MAX;
    u32::max_value();
    u8::max_value();
    u8::min_value();
    ::std::primitive::u8::MIN;
    ::std::u8::MIN;
    ::std::primitive::u8::min_value();
    std::primitive::u32::max_value();
    use std::f64;
    f64::MAX;
    // Don't lint
    f32::EPSILON;
    u8::MIN;
    external! {
        ::std::primitive::u8::MIN;
        ::std::u8::MIN;
        ::std::primitive::u8::min_value();
    }
}

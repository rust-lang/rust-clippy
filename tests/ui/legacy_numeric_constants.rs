//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::no_effect, deprecated, unused)]
#![warn(clippy::legacy_numeric_constants)]
#![feature(lint_reasons)]

#[macro_use]
extern crate proc_macros;

use std::u128 as _;
//~^ ERROR: importing legacy numeric constants
pub mod a {
    pub use std::u128;
    //~^ ERROR: importing legacy numeric constants
    //~| HELP: use the associated constants on `u128` instead at their usage
}

macro_rules! b {
    () => {
        mod b {
            use std::u32;
            fn b() {
                let x = std::u64::MAX;
            }
        }
    };
}

fn main() {
    std::f32::EPSILON;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    std::u8::MIN;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    std::usize::MIN;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    std::u32::MAX;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    core::u32::MAX;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    use std::u32::MAX;
    //~^ ERROR: importing a legacy numeric constant
    //~| HELP: use the associated constant `u32::MAX` instead at its usage
    use std::u8::MIN;
    //~^ ERROR: importing a legacy numeric constant
    //~| HELP: use the associated constant `u8::MIN` instead at its usage
    MAX;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    u32::max_value();
    //~^ ERROR: usage of a legacy numeric method
    //~| HELP: use the associated constant instead
    u8::max_value();
    //~^ ERROR: usage of a legacy numeric method
    //~| HELP: use the associated constant instead
    u8::min_value();
    //~^ ERROR: usage of a legacy numeric method
    //~| HELP: use the associated constant instead
    ::std::u8::MIN;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    ::std::primitive::u8::min_value();
    //~^ ERROR: usage of a legacy numeric method
    //~| HELP: use the associated constant instead
    std::primitive::u32::max_value();
    //~^ ERROR: usage of a legacy numeric method
    //~| HELP: use the associated constant instead
    f64::MAX;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: remove the import that brings `std::f64` into scope
    self::a::u128::MAX;
    //~^ ERROR: usage of a legacy numeric constant
    //~| HELP: use the associated constant instead
    use std::u32;
    //~^ ERROR: importing legacy numeric constants
    //~| HELP: use the associated constants on `u32` instead at their usage
    u32::MAX;
    use std::f64;
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
    //~^ ERROR: importing legacy numeric constants
    //~| HELP: use the associated constants on `u32` instead at their usage
    external! {
        ::std::primitive::u8::MIN;
        ::std::u8::MIN;
        ::std::primitive::u8::min_value();
    }
}

#[clippy::msrv = "1.42.0"]
fn msrv_too_low() {
    std::u32::MAX;
}

#[clippy::msrv = "1.43.0"]
fn msrv_juust_right() {
    std::u32::MAX;
}

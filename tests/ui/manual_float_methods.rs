//@no-rustfix: overlapping suggestions
//@aux-build:proc_macros.rs
#![feature(f16)]
#![feature(f128)]
#![allow(clippy::needless_ifs, unused)]
#![warn(clippy::manual_is_infinite, clippy::manual_is_finite)]

// Tests for f16 and f128 types

#[macro_use]
extern crate proc_macros;

fn fn_test() -> f64 {
    f64::NEG_INFINITY
}

fn fn_test_not_inf() -> f64 {
    112.0
}

fn main() {
    let x = 1.0f32;
    if x == f32::INFINITY || x == f32::NEG_INFINITY {}
    //~^ manual_is_infinite
    if x != f32::INFINITY && x != f32::NEG_INFINITY {}
    //~^ manual_is_finite
    let x = 1.0f64;
    if x == f64::INFINITY || x == f64::NEG_INFINITY {}
    //~^ manual_is_infinite
    if x != f64::INFINITY && x != f64::NEG_INFINITY {}
    //~^ manual_is_finite

    // f16 tests
    let x = 1.0f16;
    if x == f16::INFINITY || x == f16::NEG_INFINITY {}
    if x != f16::INFINITY && x != f16::NEG_INFINITY {}

    // f128 tests
    let x = 1.0f128;
    if x == f128::INFINITY || x == f128::NEG_INFINITY {}
    if x != f128::INFINITY && x != f128::NEG_INFINITY {}

    // Don't lint - f64 tests
    let x = 1.0f64;
    if x.is_infinite() {}
    if x.is_finite() {}
    if x.abs() < f64::INFINITY {}
    if f64::INFINITY > x.abs() {}
    if f64::abs(x) < f64::INFINITY {}
    if f64::INFINITY > f64::abs(x) {}
    // Is not evaluated by `clippy_utils::constant`
    if x != f64::INFINITY && x != fn_test() {}
    // Not -inf
    if x != f64::INFINITY && x != fn_test_not_inf() {}
    const {
        let x = 1.0f64;
        if x == f64::INFINITY || x == f64::NEG_INFINITY {}
        //~^ manual_is_infinite
    }
    const {
        let x = 1.0f16;
        if x == f16::INFINITY || x == f16::NEG_INFINITY {}
    }
    const {
        let x = 1.0f128;
        if x == f128::INFINITY || x == f128::NEG_INFINITY {}
    }
    const X: f64 = 1.0f64;
    if const { X == f64::INFINITY || X == f64::NEG_INFINITY } {}
    if const { X != f64::INFINITY && X != f64::NEG_INFINITY } {}
    external! {
        let x = 1.0;
        if x == f32::INFINITY || x == f32::NEG_INFINITY {}
        if x != f32::INFINITY && x != f32::NEG_INFINITY {}
    }
    with_span! {
        span
        let x = 1.0;
        if x == f32::INFINITY || x == f32::NEG_INFINITY {}
        if x != f32::INFINITY && x != f32::NEG_INFINITY {}
    }

    {
        let x = 1.0f32;
        const X: f32 = f32::INFINITY;
        const Y: f32 = f32::NEG_INFINITY;
        if x == X || x == Y {}
        if x != X && x != Y {}
    }

    {
        let x = 1.0f16;
        const X: f16 = f16::INFINITY;
        const Y: f16 = f16::NEG_INFINITY;
        if x == X || x == Y {}
        if x != X && x != Y {}
    }

    {
        let x = 1.0f128;
        const X: f128 = f128::INFINITY;
        const Y: f128 = f128::NEG_INFINITY;
        if x == X || x == Y {}
        if x != X && x != Y {}
    }
}

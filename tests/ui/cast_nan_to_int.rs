#![feature(f128)]
#![feature(f16)]
#![warn(clippy::cast_nan_to_int)]
#![allow(clippy::eq_op)]

fn main() {
    let _ = (0.0_f32 / -0.0) as usize;
    //~^ cast_nan_to_int

    let _ = (f64::INFINITY * -0.0) as usize;
    //~^ cast_nan_to_int

    let _ = (0.0 * f32::INFINITY) as usize;
    //~^ cast_nan_to_int

    let _ = (f64::INFINITY + f64::NEG_INFINITY) as usize;
    //~^ cast_nan_to_int

    let _ = (f32::INFINITY - f32::INFINITY) as usize;
    //~^ cast_nan_to_int

    let _ = (f32::INFINITY / f32::NEG_INFINITY) as usize;
    //~^ cast_nan_to_int

    let _ = f16::NAN as usize;
    //~^ cast_nan_to_int

    let _ = f128::NAN as u128;
    //~^ cast_nan_to_int

    // those won't be linted:
    let _ = (1.0_f32 / 0.0) as usize;
    let _ = (f32::INFINITY * f32::NEG_INFINITY) as usize;
    let _ = (f32::INFINITY - f32::NEG_INFINITY) as usize;
    let _ = (f64::INFINITY - 0.0) as usize;
}

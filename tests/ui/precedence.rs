#![warn(clippy::precedence)]
#![allow(unused_must_use, clippy::no_effect, clippy::unnecessary_operation)]
#![allow(clippy::identity_op)]
#![allow(clippy::eq_op)]

macro_rules! trip {
    ($a:expr) => {
        match $a & 0b1111_1111u8 {
            0 => println!("a is zero ({})", $a),
            _ => println!("a is {}", $a),
        }
    };
}

fn main() {
    1 << 2 + 3;
    //~^ ERROR: operator precedence can trip the unwary
    //~| NOTE: `-D clippy::precedence` implied by `-D warnings`
    1 + 2 << 3;
    //~^ ERROR: operator precedence can trip the unwary
    4 >> 1 + 1;
    //~^ ERROR: operator precedence can trip the unwary
    1 + 3 >> 2;
    //~^ ERROR: operator precedence can trip the unwary
    1 ^ 1 - 1;
    //~^ ERROR: operator precedence can trip the unwary
    3 | 2 - 1;
    //~^ ERROR: operator precedence can trip the unwary
    3 & 5 - 2;
    //~^ ERROR: operator precedence can trip the unwary
    -1i32.abs();
    //~^ ERROR: unary minus has lower precedence than method call
    -1f32.abs();
    //~^ ERROR: unary minus has lower precedence than method call

    // These should not trigger an error
    let _ = (-1i32).abs();
    let _ = (-1f32).abs();
    let _ = -(1i32).abs();
    let _ = -(1f32).abs();
    let _ = -(1i32.abs());
    let _ = -(1f32.abs());

    // Odd functions should not trigger an error
    let _ = -1f64.asin();
    let _ = -1f64.asinh();
    let _ = -1f64.atan();
    let _ = -1f64.atanh();
    let _ = -1f64.cbrt();
    let _ = -1f64.fract();
    let _ = -1f64.round();
    let _ = -1f64.signum();
    let _ = -1f64.sin();
    let _ = -1f64.sinh();
    let _ = -1f64.tan();
    let _ = -1f64.tanh();
    let _ = -1f64.to_degrees();
    let _ = -1f64.to_radians();

    // Chains containing any non-odd function should trigger (issue #5924)
    let _ = -1.0_f64.cos().cos();
    //~^ ERROR: unary minus has lower precedence than method call
    let _ = -1.0_f64.cos().sin();
    //~^ ERROR: unary minus has lower precedence than method call
    let _ = -1.0_f64.sin().cos();
    //~^ ERROR: unary minus has lower precedence than method call

    // Chains of odd functions shouldn't trigger
    let _ = -1f64.sin().sin();

    let b = 3;
    trip!(b * 8);
}

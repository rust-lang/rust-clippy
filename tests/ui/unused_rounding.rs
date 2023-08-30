#![warn(clippy::unused_rounding)]

fn main() {
    let _ = 1f32.ceil();
    //~^ ERROR: used the `ceil` method with a whole number float
    //~| NOTE: `-D clippy::unused-rounding` implied by `-D warnings`
    let _ = 1.0f64.floor();
    //~^ ERROR: used the `floor` method with a whole number float
    let _ = 1.00f32.round();
    //~^ ERROR: used the `round` method with a whole number float
    let _ = 2e-54f64.floor();

    // issue9866
    let _ = 3.3_f32.round();
    let _ = 3.3_f64.round();
    let _ = 3.0_f32.round();
    //~^ ERROR: used the `round` method with a whole number float

    let _ = 3_3.0_0_f32.round();
    //~^ ERROR: used the `round` method with a whole number float
    let _ = 3_3.0_1_f64.round();
}

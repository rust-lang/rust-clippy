#![feature(f128)]
#![feature(f16)]
#[allow(unused_variables, clippy::eq_op)]
#[warn(clippy::zero_divided_by_zero)]
fn main() {
    let nan = 0.0 / 0.0;
    //~^ zero_divided_by_zero

    let f64_nan = 0.0 / 0.0f64;
    //~^ zero_divided_by_zero

    let other_f64_nan = 0.0f64 / 0.0;
    //~^ zero_divided_by_zero

    let one_more_f64_nan = 0.0f64 / 0.0f64;
    //~^ zero_divided_by_zero

    let neg_nan = -0.0 / 0.0;
    //~^ zero_divided_by_zero

    let other_neg_nan = 0.0 / -0.0;
    //~^ zero_divided_by_zero

    let both_neg_nan = -0.0f32 / -0.0;
    //~^ zero_divided_by_zero

    let folded_nan = 0.0 / (2.0 - 2.0);
    //~^ zero_divided_by_zero

    let f16_nan = 0.0f16 / 0.0;
    //~^ zero_divided_by_zero

    let f128_nan = 0.0f128 / 0.0;
    //~^ zero_divided_by_zero

    let zero = 0.0;
    let other_zero = 0.0;
    let other_nan = zero / other_zero; // fine - this lint doesn't propagate constants.
    let not_nan = 2.0 / 0.0; // not an error: 2/0 = inf
    let also_not_nan = 0.0 / 2.0; // not an error: 0/2 = 0
    let f16_not_nan = 0.0f16 / 2.0;
    let f128_not_nan = 0.0f128 / 2.0;
}

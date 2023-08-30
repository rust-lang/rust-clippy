#![feature(const_fn_floating_point_arithmetic)]
#![warn(clippy::suboptimal_flops)]

/// Allow suboptimal_ops in constant context
pub const fn in_const_context() {
    let a: f64 = 1234.567;
    let b: f64 = 45.67834;
    let c: f64 = 0.0004;

    let _ = a * b + c;
    let _ = c + a * b;
}

fn main() {
    let a: f64 = 1234.567;
    let b: f64 = 45.67834;
    let c: f64 = 0.0004;
    let d: f64 = 0.0001;

    let _ = a * b + c;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    //~| NOTE: `-D clippy::suboptimal-flops` implied by `-D warnings`
    let _ = a * b - c;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = c + a * b;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = c - a * b;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = a + 2.0 * 4.0;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = a + 2. * 4.;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura

    let _ = (a * b) + c;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = c + (a * b);
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = a * b * c + d;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura

    let _ = a.mul_add(b, c) * a.mul_add(b, c) + a.mul_add(b, c) + c;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura
    let _ = 1234.567_f64 * 45.67834_f64 + 0.0004_f64;
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura

    let _ = (a * a + b).sqrt();
    //~^ ERROR: multiply and add expressions can be calculated more efficiently and accura

    // Cases where the lint shouldn't be applied
    let _ = (a * a + b * b).sqrt();
}

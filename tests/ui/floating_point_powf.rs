#![warn(clippy::suboptimal_flops, clippy::imprecise_flops)]
#![allow(clippy::unnecessary_cast)]

fn main() {
    let x = 3f32;
    let _ = 2f32.powf(x);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    //~| NOTE: `-D clippy::suboptimal-flops` implied by `-D warnings`
    let _ = 2f32.powf(3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = 2f32.powf(-3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f32::consts::E.powf(x);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f32::consts::E.powf(3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f32::consts::E.powf(-3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = x.powf(1.0 / 2.0);
    //~^ ERROR: square-root of a number can be computed more efficiently and accurately
    let _ = x.powf(1.0 / 3.0);
    //~^ ERROR: cube-root of a number can be computed more accurately
    //~| NOTE: `-D clippy::imprecise-flops` implied by `-D warnings`
    let _ = (x as f32).powf(1.0 / 3.0);
    //~^ ERROR: cube-root of a number can be computed more accurately
    let _ = x.powf(3.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(-2.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(16_777_215.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(-16_777_215.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = (x as f32).powf(-16_777_215.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = (x as f32).powf(3.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = (1.5_f32 + 1.0).powf(1.0 / 3.0);
    //~^ ERROR: cube-root of a number can be computed more accurately
    let _ = 1.5_f64.powf(1.0 / 3.0);
    //~^ ERROR: cube-root of a number can be computed more accurately
    let _ = 1.5_f64.powf(1.0 / 2.0);
    //~^ ERROR: square-root of a number can be computed more efficiently and accurately
    let _ = 1.5_f64.powf(3.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently

    // Cases where the lint shouldn't be applied
    let _ = x.powf(2.1);
    let _ = x.powf(-2.1);
    let _ = x.powf(16_777_216.0);
    let _ = x.powf(-16_777_216.0);

    let x = 3f64;
    let _ = 2f64.powf(x);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = 2f64.powf(3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = 2f64.powf(-3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f64::consts::E.powf(x);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f64::consts::E.powf(3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = std::f64::consts::E.powf(-3.1);
    //~^ ERROR: exponent for bases 2 and e can be computed more accurately
    let _ = x.powf(1.0 / 2.0);
    //~^ ERROR: square-root of a number can be computed more efficiently and accurately
    let _ = x.powf(1.0 / 3.0);
    //~^ ERROR: cube-root of a number can be computed more accurately
    let _ = x.powf(3.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(-2.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(-2_147_483_648.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    let _ = x.powf(2_147_483_647.0);
    //~^ ERROR: exponentiation with integer powers can be computed more efficiently
    // Cases where the lint shouldn't be applied
    let _ = x.powf(2.1);
    let _ = x.powf(-2.1);
    let _ = x.powf(-2_147_483_649.0);
    let _ = x.powf(2_147_483_648.0);
}

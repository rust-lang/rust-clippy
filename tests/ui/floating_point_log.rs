#![allow(dead_code, clippy::double_parens, clippy::unnecessary_cast)]
#![warn(clippy::suboptimal_flops, clippy::imprecise_flops)]

const TWO: f32 = 2.0;
const E: f32 = std::f32::consts::E;

fn check_log_base() {
    let x = 1f32;
    let _ = x.log(2f32);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    //~| NOTE: `-D clippy::suboptimal-flops` implied by `-D warnings`
    let _ = x.log(10f32);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = x.log(std::f32::consts::E);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = x.log(TWO);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = x.log(E);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = (x as f32).log(2f32);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately

    let x = 1f64;
    let _ = x.log(2f64);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = x.log(10f64);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
    let _ = x.log(std::f64::consts::E);
    //~^ ERROR: logarithm for bases 2, 10 and e can be computed more accurately
}

fn check_ln1p() {
    let x = 1f32;
    let _ = (1f32 + 2.).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    //~| NOTE: `-D clippy::imprecise-flops` implied by `-D warnings`
    let _ = (1f32 + 2.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x / 2.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x.powi(3)).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x.powi(3) / 2.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + (std::f32::consts::E - 1.0)).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x.powi(3) + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x + 2.0 + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x / 2.0 + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    // Cases where the lint shouldn't be applied
    let _ = (1.0 + x + 2.0).ln();
    let _ = (x + 1.0 + 2.0).ln();
    let _ = (x + 1.0 / 2.0).ln();
    let _ = (1.0 + x - 2.0).ln();

    let x = 1f64;
    let _ = (1f64 + 2.).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1f64 + 2.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x / 2.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (1.0 + x.powi(3)).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x.powi(3) + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x + 2.0 + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    let _ = (x / 2.0 + 1.0).ln();
    //~^ ERROR: ln(1 + x) can be computed more accurately
    // Cases where the lint shouldn't be applied
    let _ = (1.0 + x + 2.0).ln();
    let _ = (x + 1.0 + 2.0).ln();
    let _ = (x + 1.0 / 2.0).ln();
    let _ = (1.0 + x - 2.0).ln();
}

fn main() {}

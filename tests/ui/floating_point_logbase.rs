#![warn(clippy::suboptimal_flops)]
#![allow(clippy::unnecessary_cast)]

fn main() {
    let x = 3f32;
    let y = 5f32;
    let _ = x.ln() / y.ln();
    //~^ ERROR: log base can be expressed more clearly
    //~| NOTE: `-D clippy::suboptimal-flops` implied by `-D warnings`
    let _ = (x as f32).ln() / y.ln();
    //~^ ERROR: log base can be expressed more clearly
    let _ = x.log2() / y.log2();
    //~^ ERROR: log base can be expressed more clearly
    let _ = x.log10() / y.log10();
    //~^ ERROR: log base can be expressed more clearly
    let _ = x.log(5f32) / y.log(5f32);
    //~^ ERROR: log base can be expressed more clearly
    // Cases where the lint shouldn't be applied
    let _ = x.ln() / y.powf(3.2);
    let _ = x.powf(3.2) / y.powf(3.2);
    let _ = x.powf(3.2) / y.ln();
    let _ = x.log(5f32) / y.log(7f32);
}

#![warn(clippy::lossy_float_literal)]
#![allow(overflowing_literals, unused)]

fn main() {
    // Lossy whole-number float literals
    let _: f32 = 16_777_217.0; //~ lossy_float_literal
    let _: f32 = 16_777_219.0; //~ lossy_float_literal
    let _: f32 = 16_777_219.; //~ lossy_float_literal
    let _: f32 = 16_777_219.000; //~ lossy_float_literal
    let _ = 16_777_219f32; //~ lossy_float_literal
    let _: f32 = -16_777_219.0; //~ lossy_float_literal
    let _: f64 = 9_007_199_254_740_993.0; //~ lossy_float_literal
    let _: f64 = 9_007_199_254_740_993.; //~ lossy_float_literal
    let _: f64 = 9_007_199_254_740_993.00; //~ lossy_float_literal
    let _ = 9_007_199_254_740_993f64; //~ lossy_float_literal
    let _: f64 = -9_007_199_254_740_993.0; //~ lossy_float_literal

    // Lossless whole number float literals
    let _: f32 = 16_777_216.0;
    let _: f32 = 16_777_218.0;
    let _: f32 = 16_777_220.0;
    let _: f32 = -16_777_216.0;
    let _: f32 = -16_777_220.0;
    let _: f64 = 16_777_217.0;
    let _: f64 = -16_777_217.0;
    let _: f64 = 9_007_199_254_740_992.0;
    let _: f64 = -9_007_199_254_740_992.0;

    // Ignored whole number float literals
    let _: f32 = 1e25;
    let _: f32 = 1E25;
    let _: f64 = 1e99;
    let _: f64 = 1E99;
    let _: f32 = 0.1;

    const INF1: f32 = 1000000000000000000000000000000000f32;
    const NEG_INF1: f32 = -340282357000000000000000000000000000001_f32;
}

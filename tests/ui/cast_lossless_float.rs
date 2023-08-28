#![allow(clippy::no_effect, clippy::unnecessary_operation, dead_code)]
#![warn(clippy::cast_lossless)]

fn main() {
    // Test clippy::cast_lossless with casts to floating-point types
    let x0 = 1i8;
    let _ = x0 as f32;
    //~^ ERROR: casting `i8` to `f32` may become silently lossy if you later change the ty
    //~| NOTE: `-D clippy::cast-lossless` implied by `-D warnings`
    let _ = x0 as f64;
    //~^ ERROR: casting `i8` to `f64` may become silently lossy if you later change the ty
    let x1 = 1u8;
    let _ = x1 as f32;
    //~^ ERROR: casting `u8` to `f32` may become silently lossy if you later change the ty
    let _ = x1 as f64;
    //~^ ERROR: casting `u8` to `f64` may become silently lossy if you later change the ty
    let x2 = 1i16;
    let _ = x2 as f32;
    //~^ ERROR: casting `i16` to `f32` may become silently lossy if you later change the t
    let _ = x2 as f64;
    //~^ ERROR: casting `i16` to `f64` may become silently lossy if you later change the t
    let x3 = 1u16;
    let _ = x3 as f32;
    //~^ ERROR: casting `u16` to `f32` may become silently lossy if you later change the t
    let _ = x3 as f64;
    //~^ ERROR: casting `u16` to `f64` may become silently lossy if you later change the t
    let x4 = 1i32;
    let _ = x4 as f64;
    //~^ ERROR: casting `i32` to `f64` may become silently lossy if you later change the t
    let x5 = 1u32;
    let _ = x5 as f64;
    //~^ ERROR: casting `u32` to `f64` may become silently lossy if you later change the t

    // Test with casts from floating-point types
    let _ = 1.0f32 as f64;
    //~^ ERROR: casting `f32` to `f64` may become silently lossy if you later change the t
}

// The lint would suggest using `f64::from(input)` here but the `XX::from` function is not const,
// so we skip the lint if the expression is in a const fn.
// See #3656
const fn abc(input: f32) -> f64 {
    input as f64
}

// Same as the above issue. We can't suggest `::from` in const fns in impls
mod cast_lossless_in_impl {
    struct A;

    impl A {
        pub const fn convert(x: f32) -> f64 {
            x as f64
        }
    }
}

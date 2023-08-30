#![allow(clippy::no_effect, clippy::unnecessary_operation, dead_code)]
#![warn(clippy::cast_lossless)]

fn main() {
    // Test clippy::cast_lossless with casts to integer types
    let _ = 1i8 as i16;
    //~^ ERROR: casting `i8` to `i16` may become silently lossy if you later change the ty
    //~| NOTE: `-D clippy::cast-lossless` implied by `-D warnings`
    let _ = 1i8 as i32;
    //~^ ERROR: casting `i8` to `i32` may become silently lossy if you later change the ty
    let _ = 1i8 as i64;
    //~^ ERROR: casting `i8` to `i64` may become silently lossy if you later change the ty
    let _ = 1u8 as i16;
    //~^ ERROR: casting `u8` to `i16` may become silently lossy if you later change the ty
    let _ = 1u8 as i32;
    //~^ ERROR: casting `u8` to `i32` may become silently lossy if you later change the ty
    let _ = 1u8 as i64;
    //~^ ERROR: casting `u8` to `i64` may become silently lossy if you later change the ty
    let _ = 1u8 as u16;
    //~^ ERROR: casting `u8` to `u16` may become silently lossy if you later change the ty
    let _ = 1u8 as u32;
    //~^ ERROR: casting `u8` to `u32` may become silently lossy if you later change the ty
    let _ = 1u8 as u64;
    //~^ ERROR: casting `u8` to `u64` may become silently lossy if you later change the ty
    let _ = 1i16 as i32;
    //~^ ERROR: casting `i16` to `i32` may become silently lossy if you later change the t
    let _ = 1i16 as i64;
    //~^ ERROR: casting `i16` to `i64` may become silently lossy if you later change the t
    let _ = 1u16 as i32;
    //~^ ERROR: casting `u16` to `i32` may become silently lossy if you later change the t
    let _ = 1u16 as i64;
    //~^ ERROR: casting `u16` to `i64` may become silently lossy if you later change the t
    let _ = 1u16 as u32;
    //~^ ERROR: casting `u16` to `u32` may become silently lossy if you later change the t
    let _ = 1u16 as u64;
    //~^ ERROR: casting `u16` to `u64` may become silently lossy if you later change the t
    let _ = 1i32 as i64;
    //~^ ERROR: casting `i32` to `i64` may become silently lossy if you later change the t
    let _ = 1u32 as i64;
    //~^ ERROR: casting `u32` to `i64` may become silently lossy if you later change the t
    let _ = 1u32 as u64;
    //~^ ERROR: casting `u32` to `u64` may become silently lossy if you later change the t

    // Test with an expression wrapped in parens
    let _ = (1u8 + 1u8) as u16;
    //~^ ERROR: casting `u8` to `u16` may become silently lossy if you later change the ty
}

// The lint would suggest using `f64::from(input)` here but the `XX::from` function is not const,
// so we skip the lint if the expression is in a const fn.
// See #3656
const fn abc(input: u16) -> u32 {
    input as u32
}

// Same as the above issue. We can't suggest `::from` in const fns in impls
mod cast_lossless_in_impl {
    struct A;

    impl A {
        pub const fn convert(x: u32) -> u64 {
            x as u64
        }
    }
}

#[derive(PartialEq, Debug)]
#[repr(i64)]
enum Test {
    A = u32::MAX as i64 + 1,
}

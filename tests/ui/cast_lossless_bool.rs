#![allow(dead_code)]
#![warn(clippy::cast_lossless)]

fn main() {
    // Test clippy::cast_lossless with casts to integer types
    let _ = true as u8;
    //~^ ERROR: casting `bool` to `u8` is more cleanly stated with `u8::from(_)`
    //~| NOTE: `-D clippy::cast-lossless` implied by `-D warnings`
    let _ = true as u16;
    //~^ ERROR: casting `bool` to `u16` is more cleanly stated with `u16::from(_)`
    let _ = true as u32;
    //~^ ERROR: casting `bool` to `u32` is more cleanly stated with `u32::from(_)`
    let _ = true as u64;
    //~^ ERROR: casting `bool` to `u64` is more cleanly stated with `u64::from(_)`
    let _ = true as u128;
    //~^ ERROR: casting `bool` to `u128` is more cleanly stated with `u128::from(_)`
    let _ = true as usize;
    //~^ ERROR: casting `bool` to `usize` is more cleanly stated with `usize::from(_)`

    let _ = true as i8;
    //~^ ERROR: casting `bool` to `i8` is more cleanly stated with `i8::from(_)`
    let _ = true as i16;
    //~^ ERROR: casting `bool` to `i16` is more cleanly stated with `i16::from(_)`
    let _ = true as i32;
    //~^ ERROR: casting `bool` to `i32` is more cleanly stated with `i32::from(_)`
    let _ = true as i64;
    //~^ ERROR: casting `bool` to `i64` is more cleanly stated with `i64::from(_)`
    let _ = true as i128;
    //~^ ERROR: casting `bool` to `i128` is more cleanly stated with `i128::from(_)`
    let _ = true as isize;
    //~^ ERROR: casting `bool` to `isize` is more cleanly stated with `isize::from(_)`

    // Test with an expression wrapped in parens
    let _ = (true | false) as u16;
    //~^ ERROR: casting `bool` to `u16` is more cleanly stated with `u16::from(_)`
}

// The lint would suggest using `u32::from(input)` here but the `XX::from` function is not const,
// so we skip the lint if the expression is in a const fn.
// See #3656
const fn abc(input: bool) -> u32 {
    input as u32
}

// Same as the above issue. We can't suggest `::from` in const fns in impls
mod cast_lossless_in_impl {
    struct A;

    impl A {
        pub const fn convert(x: bool) -> u64 {
            x as u64
        }
    }
}

#[clippy::msrv = "1.27"]
fn msrv_1_27() {
    let _ = true as u8;
}

#[clippy::msrv = "1.28"]
fn msrv_1_28() {
    let _ = true as u8;
    //~^ ERROR: casting `bool` to `u8` is more cleanly stated with `u8::from(_)`
}

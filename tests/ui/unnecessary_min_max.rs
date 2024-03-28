#![allow(unused)]
#![warn(clippy::unnecessary_min_max)]
fn main() {
    // Both are Literals
    let _ = (-6_i32).min(9);
    let _ = (-6_i32).max(9);
    let _ = 9_u32.min(6);
    let _ = 9_u32.max(6);
    let _ = 6.min(7_u8);
    let _ = 6.max(7_u8);

    let x: i32 = 42;
    // signed MIN
    let _ = i32::MIN.min(x);
    let _ = i32::MIN.max(x);
    let _ = x.min(i32::MIN);
    let _ = x.max(i32::MIN);

    // signed MAX
    let _ = i32::MAX.min(x);
    let _ = i32::MAX.max(x);
    let _ = x.min(i32::MAX);
    let _ = x.max(i32::MAX);

    let x: u32 = 42;
    // unsigned MAX
    let _ = u32::MAX.min(x);
    let _ = u32::MAX.max(x);
    let _ = x.min(u32::MAX);
    let _ = x.max(u32::MAX);

    // unsigned MIN
    let _ = u32::MIN.min(x);
    let _ = u32::MIN.max(x);
    let _ = x.min(u32::MIN);
    let _ = x.max(u32::MIN);

    // unsigned with zero
    let _ = 0.min(x);
    let _ = 0.max(x);
    let _ = x.min(0_u32);
    let _ = x.max(0_u32);

    // The below cases shouldn't be lint
    let mut min = u32::MAX;
    for _ in 0..1000 {
        min = min.min(random_u32());
    }

    const I32_MIN_ISIZE: isize = i32::MIN as isize;
    let x: isize = 42;
    let _ = I32_MIN_ISIZE.min(x);
    let _ = I32_MIN_ISIZE.max(x);
    let _ = x.min(I32_MIN_ISIZE);
    let _ = x.max(I32_MIN_ISIZE);
}
fn random_u32() -> u32 {
    // random number generator
    0
}

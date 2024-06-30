#![warn(clippy::manual_div_ceil)]
#![feature(int_roundings)]

fn main() {
    let x = 7_i32;
    let y = 4_i32;
    let z_u: u32 = 11;

    // Lint.
    let _ = (x + (y - 1)) / y;
    let _ = ((y - 1) + x) / y;
    let _ = (x + y - 1) / y;

    let _ = (7_i32 + (4 - 1)) / 4;

    let _ = (z_u as i32 + (y - 1)) / y;
    let _ = (7_u32 as i32 + (y - 1)) / y;

    // No lint.
    let _ = (x + (y - 2)) / y;
    let _ = (x + (y + 1)) / y;

    let z = 3_i32;
    let _ = (x + (y - 1)) / z;
}

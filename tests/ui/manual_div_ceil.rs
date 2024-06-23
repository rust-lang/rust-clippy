#![warn(clippy::manual_div_ceil)]
#![feature(int_roundings)]

fn main() {
    let x: i32 = 7;
    let y: i32 = 4;

    // Lint.
    let _ = (x + (y - 1)) / y;
    let _ = ((y - 1) + x) / y;
    let _ = (x + y - 1) / y;

    let _ = (7 + (4 - 1)) / 4;

    // No lint.
    let _ = (x + (y - 2)) / y;
    let _ = (x + (y + 1)) / y;

    let z: i32 = 3;
    let _ = (x + (y - 1)) / z;
}

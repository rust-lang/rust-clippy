#![warn(clippy::unsigned_subtraction_gt_zero)]
#![allow(clippy::needless_if)]

fn main() {
    let (a, b): (u32, u32) = (1, 2);
    if a - b > 0 {}
    //~^ unsigned_subtraction_gt_zero

    if 0 < a - b {}
    //~^ unsigned_subtraction_gt_zero

    let (x, y): (usize, usize) = (10, 3);
    if x - y > 0 {}
    //~^ unsigned_subtraction_gt_zero

    if 0 < x - y {}
    //~^ unsigned_subtraction_gt_zero

    // signed: should not lint
    let (i, j): (i32, i32) = (1, 2);
    if i - j > 0 {}

    // float: should not lint
    let (f, g): (f32, f32) = (1.0, 2.0);
    if f - g > 0.0 {}

    // using saturating_sub: should not lint
    if a.saturating_sub(b) > 0 {}
}

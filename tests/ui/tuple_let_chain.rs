#![warn(clippy::tuple_let_chain)]
#![allow(clippy::redundant_pattern_matching)]

fn main() {
    let x = Some(1);
    let y = Ok::<i32, i32>(2);
    let a = Some(3);
    let b = None::<i32>;

    // Should fail
    if let (Some(x), Ok(y)) = (x, y) {}
    if let [Some(_), None] = [a, b] {}
    if let (Some(x), Ok(y)) = (x, y)
        && let [Some(_), None] = [a, b]
    {}

    // Should NOT fail
    let c = Some(1);
    let d = Some(2);
    let e = Some(3);

    // Swapped variables
    if let (Some(c), Some(d)) = (d, c) {}

    // Different variables
    if let (Some(c), Some(d)) = (c, e) {}
}

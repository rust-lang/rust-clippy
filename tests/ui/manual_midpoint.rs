#![warn(clippy::manual_midpoint)]

macro_rules! mac {
    ($a: expr, $b: expr) => {{ ($a + $b) / 2 }};
}

fn main() {
    let a: u32 = 10;
    let _ = (a + 5) / 2; //~ ERROR: manual implementation of `midpoint`

    let f: f32 = 10.0;
    let _ = (f + 5.0) / 2.0; //~ ERROR: manual implementation of `midpoint`

    // Do not lint if a literal is not present
    let _ = (f + 5.0) / (1.0 + 1.0);

    // Do not lint on signed integer types
    let i: i32 = 10;
    let _ = (i + 5) / 2;

    // Do not lint
    let _ = mac!(10, 20);
}

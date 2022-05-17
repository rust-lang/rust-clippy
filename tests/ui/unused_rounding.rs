// run-rustfix
#![warn(clippy::unnecessary_rounding)]

fn main() {
    let _ = 1f32.ceil();
    let _ = 1.0f64.floor();
    let _ = 1.00f32.round();
}

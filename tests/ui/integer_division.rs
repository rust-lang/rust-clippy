#![warn(clippy::integer_division)]

const _: usize = 64 / 3;

fn main() {
    let two = 2;
    let n = 1 / 2;
    let o = 1 / two;
    let p = two / 4;
    let x = 1. / 2.0;
}

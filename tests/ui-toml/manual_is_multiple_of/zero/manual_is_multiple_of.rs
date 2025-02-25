#![warn(clippy::manual_is_multiple_of)]

fn main() {}

fn f(a: u64, b: u64) {
    let _ = a & 1 == 0; //~ manual_is_multiple_of
    let _ = a & ((1 << 1) - 1) == 0; //~ manual_is_multiple_of
}

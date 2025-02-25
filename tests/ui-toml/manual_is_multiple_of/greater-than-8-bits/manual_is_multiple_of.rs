#![warn(clippy::manual_is_multiple_of)]

fn main() {}

fn f(a: u64, b: u64) {
    let _ = a & 0xff == 0;
    let _ = a & 0x1ff == 0; //~ manual_is_multiple_of
}

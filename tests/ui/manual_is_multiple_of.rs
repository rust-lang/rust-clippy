#![warn(clippy::manual_is_multiple_of)]

fn main() {}

#[clippy::msrv = "1.87"]
fn f(a: u64, b: u64) {
    let _ = a % b == 0; //~ manual_is_multiple_of
    let _ = (a + 1) % (b + 1) == 0; //~ manual_is_multiple_of
    let _ = a % b != 0; //~ manual_is_multiple_of
    let _ = (a + 1) % (b + 1) != 0; //~ manual_is_multiple_of

    let _ = a & 4095 == 0; //~ manual_is_multiple_of
    let _ = a & 4095 != 0; //~ manual_is_multiple_of
    let _ = a & ((1 << b) - 1) == 0; //~ manual_is_multiple_of
    let _ = a & ((1 << b) - 1) != 0; //~ manual_is_multiple_of
    let _ = ((1 << b) - 1) & a == 0; //~ manual_is_multiple_of

    let _ = a % b > 0; //~ manual_is_multiple_of
    let _ = 0 < a % b; //~ manual_is_multiple_of

    let _ = a & 0xff == 0; //~ manual_is_multiple_of

    let _ = a & 1 == 0; // Do not lint: below `min-and-mask-size`
    let _ = a & ((1 << 1) - 1) == 0; // Do not lint: below `min-and-mask-size`
    let _ = a & 7 == 0; //~ manual_is_multiple_of
    let _ = a & ((1 << 3) - 1) == 0; //~ manual_is_multiple_of
}

#[clippy::msrv = "1.86"]
fn g(a: u64, b: u64) {
    let _ = a % b == 0;
}

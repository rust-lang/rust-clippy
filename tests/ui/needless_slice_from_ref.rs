// run-rustfix

#![warn(clippy::needless_slice_from_ref)]

fn main() {
    let x = 3;
    let _s = core::slice::from_ref(&x);
    let _s = std::slice::from_ref(&x);
}

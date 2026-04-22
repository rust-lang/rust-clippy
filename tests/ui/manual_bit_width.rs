#![warn(clippy::manual_bit_width)]

fn main() {
    let x: u32 = 5;

    // `T::BITS - x.leading_zeros()`
    let _ = u32::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();
}

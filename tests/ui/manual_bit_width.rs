#![warn(clippy::manual_bit_width)]

fn main() {
    let x: u32 = 5;

    // `T::BITS - x.leading_zeros()`
    let _ = u8::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u16::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u32::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u64::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = usize::BITS - x.leading_zeros(); //~ manual_bit_width

    // negative cases.
    let _ = 128 - x.leading_zeros();

    // signed integers do not implement `bit_width()`
    let _ = i8::BITS - x.leading_zeros();
    let _ = i16::BITS - x.leading_zeros();
    let _ = i32::BITS - x.leading_zeros();
    let _ = i64::BITS - x.leading_zeros();
}

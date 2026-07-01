#![warn(clippy::mismatched_bit_width_type)]

use core::num::{self, NonZero};

fn main() {
    // left and right expression have different calling types
    let x: u32 = 5;

    let _ = u8::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = i16::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = NonZero::<u64>::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = NonZero::<isize>::BITS - x.leading_zeros(); //~ mismatched_bit_width_type

    // negative case.
    // left expression is a literal
    let z: u32 = 1_000_000 - x.leading_zeros();
}

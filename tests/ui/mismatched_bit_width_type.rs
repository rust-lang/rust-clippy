#![warn(clippy::mismatched_bit_width_type)]

use core::num::{self, NonZero, NonZeroI32, NonZeroU32};

fn main() {
    // left and right expression have different calling types
    let w: u32 = 5;
    let _ = u64::BITS - w.leading_zeros(); //~ mismatched_bit_width_type
    let x: i64 = -5;
    let _ = u16::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let y = NonZero::<u64>::new(5).unwrap();
    let _ = i32::BITS - y.leading_zeros(); //~ mismatched_bit_width_type
    let z = NonZero::<isize>::new(5).unwrap();
    let _ = NonZeroU32::BITS - z.leading_zeros(); //~ mismatched_bit_width_type

    // negative case.
    // left expression is a literal
    let z: u32 = 1_000_000 - x.leading_zeros();
}

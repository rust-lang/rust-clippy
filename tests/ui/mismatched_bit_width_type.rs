#![warn(clippy::mismatched_bit_width_type)]

use core::num::{self, NonZero, NonZeroI32, NonZeroU32};

fn main() {
    let x: u32 = 5;

    let _ = u8::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = u16::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = u64::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = u128::BITS - x.leading_zeros(); //~ mismatched_bit_width_type

    let _ = i8::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = i16::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = i32::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = i64::BITS - x.leading_zeros(); //~ mismatched_bit_width_type

    let x = NonZeroU32::new(5).unwrap();
    let _ = u32::BITS - x.leading_zeros(); //~ mismatched_bit_width_type
    let _ = NonZeroI32::BITS - x.leading_zeros(); //~ mismatched_bit_width_type

    // negative cases
    // where types matches
    let _ = i32::BITS - 5i32.leading_zeros();
    let _ = u32::BITS - 5u32.leading_zeros();
    let _ = usize::BITS - 5usize.leading_zeros();
    let _ = NonZeroU32::BITS - x.leading_zeros();

    // where type mismatches but x is signed
    let _ = i32::BITS - 5i32.leading_zeros();
    let _ = u32::BITS - 5i32.leading_zeros();
    let _ = usize::BITS - 5isize.leading_zeros();
    let _ = NonZeroU32::BITS - NonZeroI32::new(5).unwrap().leading_zeros();
}

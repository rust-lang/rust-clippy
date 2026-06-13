#![warn(clippy::manual_bit_width)]

use core::num::{self, NonZero, NonZeroU32};

fn main() {
    let x: u8 = 5;

    // `T::BITS - x.leading_zeros()`
    let _ = u8::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u16::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u32::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u64::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = usize::BITS - x.leading_zeros(); //~ manual_bit_width

    // `NonZero::<T>::BITS - x.leading_zeros()`
    let y = NonZero::<u8>::new(5).unwrap();
    let _ = NonZero::<u8>::BITS - y.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<u16>::BITS - y.leading_zeros(); //~ manual_bit_width
    let _ = NonZeroU32::BITS - y.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<u64>::BITS - y.leading_zeros(); //~ manual_bit_width
    let _ = num::NonZero::<usize>::BITS - y.leading_zeros(); //~ manual_bit_width

    // negative cases.
    let _ = 128 - x.leading_zeros();

    // signed integers do not implement `bit_width()`
    let _ = i8::BITS - x.leading_zeros();
    let _ = i16::BITS - x.leading_zeros();
    let _ = i32::BITS - x.leading_zeros();
    let _ = i64::BITS - x.leading_zeros();
}

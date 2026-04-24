#![warn(clippy::manual_bit_width)]

use core::num::NonZero;
use std::num;

fn main() {
    let x: u32 = 5;

    // `T::BITS - x.leading_zeros()`
    let _ = u8::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u16::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u32::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = u64::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = usize::BITS - x.leading_zeros(); //~ manual_bit_width

    // `NonZero::<T>::BITS - x.leading_zeros()`
    let _ = NonZero::<u8>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<u16>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<u32>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<u64>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = NonZero::<usize>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = num::NonZero::<usize>::BITS - x.leading_zeros(); //~ manual_bit_width

    let _ = 128 - x.leading_zeros();
}

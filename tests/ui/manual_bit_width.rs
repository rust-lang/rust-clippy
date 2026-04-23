#![warn(clippy::manual_bit_width)]

use core::num::NonZero;
use std::num;

fn main() {
    // `T::BITS - x.leading_zeros()`
    let x: u32 = 5;
    let _ = u32::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    // `NonZero::<T>::BITS - x.leading_zeros()`
    let x = NonZero::<u8>::new(0b1101).unwrap();
    let _ = NonZero::<u8>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    let x = NonZero::<u16>::new(0b1101).unwrap();
    let _ = NonZero::<u16>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    let x = NonZero::<u32>::new(0b1010).unwrap();
    let _ = NonZero::<u32>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    let x = NonZero::<u64>::new(0b1110).unwrap();
    let _ = NonZero::<u64>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    let x = NonZero::<usize>::new(0b0111).unwrap();
    let _ = NonZero::<usize>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();

    let x = num::NonZero::<usize>::new(0b0111).unwrap();
    let _ = num::NonZero::<usize>::BITS - x.leading_zeros(); //~ manual_bit_width
    let _ = 128 - x.leading_zeros();
}

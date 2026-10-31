#![warn(clippy::transmute_int_to_non_zero)]
#![allow(clippy::useless_nonzero_new_unchecked)]

use core::num::NonZero;

const NON_NULL_U8: NonZero<u8> = unsafe { std::mem::transmute(1u8) };
//~^ transmute_int_to_non_zero

const NON_NULL_USIZE: NonZero<usize> = unsafe { std::mem::transmute(1usize) };
//~^ transmute_int_to_non_zero

fn main() {}

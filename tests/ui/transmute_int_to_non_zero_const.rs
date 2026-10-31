//@check-pass
#![warn(clippy::transmute_int_to_non_zero)]

use std::num::{NonZeroU8, NonZeroUsize};

const NON_NULL_U8: NonZeroU8 = unsafe { std::mem::transmute(1u8) };
const NON_NULL_USIZE: NonZeroUsize = unsafe { std::mem::transmute(1usize) };

fn main() {}

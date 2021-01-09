#![allow(unused)]
#![warn(clippy::int_min_max_value)]

use std::i32;

type Uint = u8;

fn main() {
    let min = u32::min_value();
    let max = isize::max_value();
    let min = crate::Uint::min_value();
    let max = std::u32::MAX;

    let min = isize::MIN;
    let max = usize::MAX;
}

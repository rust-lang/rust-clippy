// run-rustfix

#![allow(unused)]
#![warn(clippy::int_min_max_value)]

type Uint = u8;

fn main() {
    let min = u32::min_value();
    let max = isize::max_value();
    let min = crate::Uint::min_value();
}

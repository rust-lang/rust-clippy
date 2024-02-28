#![warn(clippy::usize_unportable_32_bit_literal)]
#![allow(clippy::no_effect)]

fn main() {
    // assignment of value larger than 32 bit can express
    let _: usize = 4294967296;

    // single literals are also problematic
    4294967297usize;

    // A counter example
    let _: usize = 1;

    // u32::MAX shouldn't be a problem
    let _: usize = 4_294_967_295usize;

    // if the code only runs on 64 bit platforms it's not a problem
    #[cfg(target_pointer_width = "64")]
    let _: usize = 4294967296;

    // single literals on 64bit platforms only is not a problem either
    #[cfg(target_pointer_width = "64")]
    4294967297usize;
}

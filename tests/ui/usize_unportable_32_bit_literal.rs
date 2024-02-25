#![warn(clippy::usize_unportable_32_bit_literal)]

fn main() {
    // assignment of value larger than 32 bit can express
    let _: usize = 4294967296;

    // A counter example
    let _: usize = 1;

    // u32::MAX shouldn't be a problem
    let _: usize = 4_294_967_295usize;

    // if the code only runs on 64 bit platforms it's not a problem
    #[cfg(target_pointer_width = "64")]
    let _: usize = 4294967296;
}

#![warn(clippy::manual_bits)]
#![allow(
    clippy::no_effect,
    clippy::useless_conversion,
    path_statements,
    unused_must_use,
    clippy::unnecessary_operation,
    clippy::unnecessary_cast
)]

use std::mem::{size_of, size_of_val};

fn main() {
    size_of::<i8>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    //~| NOTE: `-D clippy::manual-bits` implied by `-D warnings`
    size_of::<i16>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<i32>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<i64>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<i128>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<isize>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits

    size_of::<u8>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<u16>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<u32>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<u64>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<u128>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    size_of::<usize>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits

    8 * size_of::<i8>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<i16>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<i32>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<i64>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<i128>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<isize>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits

    8 * size_of::<u8>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<u16>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<u32>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<u64>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<u128>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    8 * size_of::<usize>();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits

    size_of::<usize>() * 4;
    4 * size_of::<usize>();
    size_of::<bool>() * 8;
    8 * size_of::<bool>();

    size_of_val(&0u32) * 8;

    type Word = u32;
    size_of::<Word>() * 8;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    type Bool = bool;
    size_of::<Bool>() * 8;

    let _: u32 = (size_of::<u128>() * 8) as u32;
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    let _: u32 = (size_of::<u128>() * 8).try_into().unwrap();
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    let _ = (size_of::<u128>() * 8).pow(5);
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
    let _ = &(size_of::<u128>() * 8);
    //~^ ERROR: usage of `mem::size_of::<T>()` to obtain the size of `T` in bits
}

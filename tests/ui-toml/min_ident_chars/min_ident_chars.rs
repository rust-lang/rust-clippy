//@aux-build:extern_types.rs
#![allow(nonstandard_style, unused)]
#![warn(clippy::min_ident_chars)]

extern crate extern_types;
use extern_types::{Aaa, LONGER, M, N as W};
//~^ min_ident_chars

pub const N: u32 = 0;
//~^ min_ident_chars
pub const LONG: u32 = 32;

struct Owo {
    Uwu: u128,
    aaa: Aaa,
    //~^ min_ident_chars
}

fn main() {
    let wha = 1;
    let vvv = 1;
    //~^ min_ident_chars
    let uuu = 1;
    //~^ min_ident_chars
    let (mut a, mut b) = (1, 2);
    //~^ min_ident_chars
    //~| min_ident_chars
    for i in 0..1000 {}
    //~^ min_ident_chars
}

trait ShortParams {
    fn f(g: i32);
    //~^ min_ident_chars
    //~| min_ident_chars
    fn long_name(long_param: i32);
}

struct MyStruct;

impl ShortParams for MyStruct {
    fn f(g: i32) {}
    //~^ min_ident_chars
    //~| min_ident_chars
    fn long_name(long_param: i32) {}
}

impl core::fmt::Display for MyStruct {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        //~^ min_ident_chars
        //~| min_ident_chars
        //~| min_ident_chars
        write!(f, "MyStruct")
    }
}

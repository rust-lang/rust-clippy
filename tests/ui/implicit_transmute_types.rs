//@run-rustfix
#![allow(unused)]
#![warn(clippy::implicit_transmute_types)]

use std::mem;

fn main() {
    unsafe {
        let _: u32 = mem::transmute(123i32);

        let _: u32 = mem::transmute::<i32, u32>(123i32);
        let _: u32 = mem::transmute::<_, _>(123i32);
    }
}

#![feature(plugin)]
#![plugin(clippy)]

use std::{mem, ptr};

#[test]
fn match_transmute_write() {
    match Some(()) {
        Some(_) => unsafe { ptr::write(ptr::null_mut() as *mut u32, mem::transmute::<[u8; 4], _>([0, 0, 0, 255])) },
        None => unsafe { ptr::write(ptr::null_mut() as *mut u32, mem::transmute::<[u8; 4], _>([13, 246, 24, 255])) },
    }
}

fn main() {}

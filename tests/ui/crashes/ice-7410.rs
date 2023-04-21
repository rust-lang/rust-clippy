//@compile-flags: -Clink-arg=-nostartfiles
//@ignore-macos
//@ignore-windows

#![feature(lang_items, start, libc)]
#![no_std]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::redundant_pattern_matching)]

use core::panic::PanicInfo;

struct S;

impl Drop for S {
    fn drop(&mut self) {}
}

#[start]
fn main(argc: isize, argv: *const *const u8) -> isize {
    if let Some(_) = Some(S) {
    } else {
    }
    0
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

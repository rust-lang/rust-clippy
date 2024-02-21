#![feature(lang_items, start, libc)]
#![no_std]
#![deny(clippy::zero_ptr)]

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let _ = 0 as *const usize; //~ zero_ptr
    let _ = 0 as *mut f64; //~ zero_ptr
    let _: *const u8 = 0 as *const _; //~ zero_ptr
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

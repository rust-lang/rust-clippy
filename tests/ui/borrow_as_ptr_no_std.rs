#![warn(clippy::borrow_as_ptr)]
#![feature(lang_items, start, libc)]
#![no_std]

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let val = 1;
    let _p = &val as *const i32;
    //~^ ERROR: borrow as raw pointer
    //~| NOTE: `-D clippy::borrow-as-ptr` implied by `-D warnings`

    let mut val_mut = 1;
    let _p_mut = &mut val_mut as *mut i32;
    //~^ ERROR: borrow as raw pointer
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

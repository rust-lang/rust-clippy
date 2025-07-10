#![no_std]
#![no_main]
#![warn(clippy::large_enum_variant)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

enum Myenum { //~ ERROR: large size difference between variants
    Small(u8),
    Large([u8;1024]),
} 

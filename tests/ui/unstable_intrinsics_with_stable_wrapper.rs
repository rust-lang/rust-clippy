#![warn(clippy::unstable_intrinsics_with_stable_wrapper)]
#![allow(clippy::invalid_null_ptr_usage)]
#![feature(core_intrinsics)]

use core::intrinsics::{add_with_overflow, assert_inhabited};
//~^ ERROR: consider using the stable counterpart
use std::ptr::write_bytes;

fn main() {
    // Shouldn't warn since we can't infer just from its name if it's an intrinsic or not.
    add_with_overflow(12, 14);
    // There is a stable counterpart so it should warn.
    core::intrinsics::add_with_overflow(12, 14);
    //~^ ERROR: consider using the stable counterpart
    // This one doesn't have a counterpart so should not emit a warning.
    core::intrinsics::assert_inhabited::<usize>();
    assert_inhabited::<usize>();
    // Shouldn't warn because it's the safe version.
    unsafe {
        std::ptr::write_bytes::<usize>(std::ptr::null_mut(), 42, 0);
        write_bytes::<usize>(std::ptr::null_mut(), 42, 0);
        std::intrinsics::write_bytes::<usize>(std::ptr::null_mut(), 42, 0);
    }
}

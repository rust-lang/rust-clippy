#![warn(clippy::swap_ptr_to_ref)]

use core::ptr::addr_of_mut;

fn main() {
    let mut x = 0u32;
    let y: *mut _ = &mut x;
    let z: *mut _ = &mut x;

    unsafe {
        core::mem::swap(&mut *y, &mut *z);
        //~^ ERROR: call to `core::mem::swap` with a parameter derived from a raw pointer
        //~| NOTE: `-D clippy::swap-ptr-to-ref` implied by `-D warnings`
        core::mem::swap(&mut *y, &mut x);
        //~^ ERROR: call to `core::mem::swap` with a parameter derived from a raw pointer
        core::mem::swap(&mut x, &mut *y);
        //~^ ERROR: call to `core::mem::swap` with a parameter derived from a raw pointer
        core::mem::swap(&mut *addr_of_mut!(x), &mut *addr_of_mut!(x));
        //~^ ERROR: call to `core::mem::swap` with a parameter derived from a raw pointer
    }

    let y = &mut x;
    let mut z = 0u32;
    let z = &mut z;

    core::mem::swap(y, z);
}

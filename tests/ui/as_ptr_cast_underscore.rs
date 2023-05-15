#![warn(clippy::as_ptr_cast_underscore)]
#![allow(dead_code)]
#![allow(unused)]

static X: u32 = 0u32;

fn takes_raw(a: *mut u32) -> *mut u32 {
    a
}

fn return_raw() -> *mut u32 {
    // UB but i don't care
    X as *mut u32
}

fn main() {
    let e = return_raw() as *mut _;
    let e = return_raw() as *const _;
    let mut z = vec![11; 100];
    let z = z.as_mut_ptr() as *mut _;
    // since this changes its type, this will not be linted
    takes_raw(z);

    // this doesn't, however, so lint this
    let mut z = [1u32; 2];
    let z = z.as_mut_ptr() as *mut _;
    let z = takes_raw(z as *mut _) as *mut _;

    let mut x = [3; 11];
    // do not lint if it changes type
    let z: *const u32 = x.as_ptr() as *const _;
    // lint this
    let w = x.as_ptr() as *const _;
    let x = x.as_mut_ptr() as *mut _;

    // this is not a raw pointer. do not lint this.
    let y = 1i32;
    let y = y as i64;
}

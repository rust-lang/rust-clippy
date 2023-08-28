#![allow(unused)]
#![allow(clippy::useless_vec)]

use std::collections::{HashSet, VecDeque};

fn main() {
    let v = [1, 2, 3, 4, 5];
    let v2: Vec<isize> = v.iter().cloned().collect();
    //~^ ERROR: called `iter().cloned().collect()` on a slice to create a `Vec`. Calling `
    //~| NOTE: `-D clippy::iter-cloned-collect` implied by `-D warnings`
    let v3: HashSet<isize> = v.iter().cloned().collect();
    let v4: VecDeque<isize> = v.iter().cloned().collect();

    // Handle macro expansion in suggestion
    let _: Vec<isize> = vec![1, 2, 3].iter().cloned().collect();
    //~^ ERROR: called `iter().cloned().collect()` on a slice to create a `Vec`. Calling `

    // Issue #3704
    unsafe {
        let _: Vec<u8> = std::ffi::CStr::from_ptr(std::ptr::null())
            .to_bytes()
            //~^ ERROR: called `iter().cloned().collect()` on a slice to create a `Vec`. C
            .iter()
            .cloned()
            .collect();
    }

    // Issue #6808
    let arr: [u8; 64] = [0; 64];
    let _: Vec<_> = arr.iter().cloned().collect();
    //~^ ERROR: called `iter().cloned().collect()` on a slice to create a `Vec`. Calling `

    // Issue #6703
    let _: Vec<isize> = v.iter().copied().collect();
    //~^ ERROR: called `iter().copied().collect()` on a slice to create a `Vec`. Calling `
}

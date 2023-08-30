#![warn(clippy::deref_by_slicing)]
#![allow(clippy::borrow_deref_ref)]

use std::io::Read;

fn main() {
    let mut vec = vec![0];
    let _ = &vec[..];
    //~^ ERROR: slicing when dereferencing would work
    //~| NOTE: `-D clippy::deref-by-slicing` implied by `-D warnings`
    let _ = &mut vec[..];
    //~^ ERROR: slicing when dereferencing would work

    let ref_vec = &mut vec;
    let _ = &ref_vec[..];
    //~^ ERROR: slicing when dereferencing would work
    let mut_slice = &mut ref_vec[..];
    //~^ ERROR: slicing when dereferencing would work
    let _ = &mut mut_slice[..]; // Err, re-borrows slice
    //~^ ERROR: slicing when dereferencing would work

    let s = String::new();
    let _ = &s[..];
    //~^ ERROR: slicing when dereferencing would work

    static S: &[u8] = &[0, 1, 2];
    let _ = &mut &S[..]; // Err, re-borrows slice
    //~^ ERROR: slicing when dereferencing would work

    let slice: &[u32] = &[0u32, 1u32];
    let slice_ref = &slice;
    let _ = &slice_ref[..]; // Err, derefs slice
    //~^ ERROR: slicing when dereferencing would work

    let bytes: &[u8] = &[];
    let _ = (&bytes[..]).read_to_end(&mut vec![]).unwrap(); // Err, re-borrows slice
    //~^ ERROR: slicing when dereferencing would work
}

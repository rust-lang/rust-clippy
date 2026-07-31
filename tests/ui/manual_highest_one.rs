#![warn(clippy::manual_highest_one)]
// FIXME: remove this after this lint is fixed
#![allow(clippy::mismatched_bit_width_type)]

use std::num::NonZeroU32;

fn main() {
    let u = 5_u32;
    let nz = NonZeroU32::new(5).unwrap();

    // basic cases: suggest `u.highest_one()`
    let _ = 31 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - 1 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - u.leading_zeros() - 1; //~ manual_highest_one
    let _ = u32::BITS - (1 + u.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS - (u.leading_zeros() + 1); //~ manual_highest_one

    // nonzero: suggest `u.highest_one()`
    let _ = 31 - nz.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - 1 - nz.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - nz.leading_zeros() - 1; //~ manual_highest_one
    let _ = u32::BITS - (1 + nz.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS - (nz.leading_zeros() + 1); //~ manual_highest_one

    // if block: suggest `u.highest_one().unwrap_or($1)`
    let _ = if u == 0 {
        //~^ manual_highest_one
        0
    } else {
        31 - u.leading_zeros()
    };
    let _ = if 0 == u {
        //~^ manual_highest_one
        1
    } else {
        31 - u.leading_zeros()
    };
    let _ = if u != 0 {
        //~^ manual_highest_one
        31 - u.leading_zeros()
    } else {
        2
    };
    let _ = if 0 != u {
        //~^ manual_highest_one
        31 - u.leading_zeros()
    } else {
        let _ = "Rust is fast, memory-safe and productive.";
        3
    };
    // FIXME: this lint cannot capture this
    let _ = if u == 0 {
        let v = u;
        31 - v.leading_zeros() //~ manual_highest_one
    } else {
        let _ = "Rust is fast, memory-safe and productive.";
        3
    };

    macro_rules! ignore_me {
        () => {
            let _ = 31 - u.leading_zeros();
            let _ = u32::BITS - 1 - u.leading_zeros();
            let _ = u32::BITS - (1 + u.leading_zeros());
        };
    }
    ignore_me!();
}

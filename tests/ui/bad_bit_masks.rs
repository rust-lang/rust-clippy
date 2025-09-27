//@aux-build:proc_macros.rs

#![warn(clippy::bad_bit_mask)]
#![allow(clippy::erasing_op, clippy::ineffective_bit_mask, clippy::identity_op)]

use core::hint::black_box;
use proc_macros::{external, with_span};

fn main() {
    let x = black_box(5u32);

    let _ = x & 0b0000 == 0b0000; //~ bad_bit_mask
    let _ = x & 0b0000 == 0b0001; //~ bad_bit_mask
    let _ = x & 0b0001 == 0b0000;
    let _ = x & 0b0001 == 0b0001;
    let _ = x & 0b0010 == 0b0001; //~ bad_bit_mask
    let _ = x & 0b0001 == 0b0010; //~ bad_bit_mask
    let _ = x & 0b0011 == 0b0001;
    let _ = x & 0b0011 == 0b0010;
    let _ = x & 0b0011 == 0b0011;
    let _ = x & 0b0011 == 0b0100; //~ bad_bit_mask
    let _ = x & 0b0100 == 0b0100;
    let _ = x & 0b0110 == 0b0111; //~ bad_bit_mask
    let _ = x & 0b0011 == 0b0101; //~ bad_bit_mask
    let _ = x & 0b1111 == 0b1110;
    let _ = x & 0b1111 == 0b1010;

    let _ = x | 0b0000 == 0b0000;
    let _ = x | 0b0000 == 0b0001;
    let _ = x | 0b0001 == 0b0000; //~ bad_bit_mask
    let _ = x | 0b0001 == 0b0001;
    let _ = x | 0b0010 == 0b0001; //~ bad_bit_mask
    let _ = x | 0b0001 == 0b0010; //~ bad_bit_mask
    let _ = x | 0b0011 == 0b0001; //~ bad_bit_mask
    let _ = x | 0b0011 == 0b0010; //~ bad_bit_mask
    let _ = x | 0b0011 == 0b0011;
    let _ = x | 0b0011 == 0b0100; //~ bad_bit_mask
    let _ = x | 0b0100 == 0b0100;
    let _ = x | 0b0110 == 0b0111;
    let _ = x | 0b0011 == 0b0101; //~ bad_bit_mask
    let _ = x | 0b1111 == 0b1110; //~ bad_bit_mask
    let _ = x | 0b1111 == 0b1010; //~ bad_bit_mask

    let _ = x & 0b0000 != 0b0000; //~ bad_bit_mask
    let _ = x & 0b0000 != 0b0001; //~ bad_bit_mask
    let _ = x & 0b0001 != 0b0000;
    let _ = x & 0b0001 != 0b0001;
    let _ = x & 0b0010 != 0b0001; //~ bad_bit_mask
    let _ = x & 0b0001 != 0b0010; //~ bad_bit_mask
    let _ = x & 0b0011 != 0b0001;
    let _ = x & 0b0011 != 0b0010;
    let _ = x & 0b0011 != 0b0011;
    let _ = x & 0b0011 != 0b0100; //~ bad_bit_mask
    let _ = x & 0b0100 != 0b0100;
    let _ = x & 0b0110 != 0b0111; //~ bad_bit_mask
    let _ = x & 0b0011 != 0b0101; //~ bad_bit_mask
    let _ = x & 0b1111 != 0b1110;
    let _ = x & 0b1111 != 0b1010;

    let _ = x | 0b0000 != 0b0000;
    let _ = x | 0b0000 != 0b0001;
    let _ = x | 0b0001 != 0b0000; //~ bad_bit_mask
    let _ = x | 0b0001 != 0b0001;
    let _ = x | 0b0010 != 0b0001; //~ bad_bit_mask
    let _ = x | 0b0001 != 0b0010; //~ bad_bit_mask
    let _ = x | 0b0011 != 0b0001; //~ bad_bit_mask
    let _ = x | 0b0011 != 0b0010; //~ bad_bit_mask
    let _ = x | 0b0011 != 0b0011;
    let _ = x | 0b0011 != 0b0100; //~ bad_bit_mask
    let _ = x | 0b0100 != 0b0100;
    let _ = x | 0b0110 != 0b0111;
    let _ = x | 0b0011 != 0b0101; //~ bad_bit_mask
    let _ = x | 0b1111 != 0b1110; //~ bad_bit_mask
    let _ = x | 0b1111 != 0b1010; //~ bad_bit_mask

    let _ = 0b0010 & x == 0b0001; //~ bad_bit_mask
    let _ = 0b0001 == x & 0b0010; //~ bad_bit_mask
    let _ = 0b0001 == 0b0010 & x; //~ bad_bit_mask

    let _ = x & (0b0100 | 0b0010) == (0b0111 ^ 0b1000); //~ bad_bit_mask

    external! {
        let x = black_box(5u32);
        let _ = x & 0b0010 == 0b0001;
    }
    with_span! {
        sp
        let x = black_box(5u32);
        let _ = x & 0b0010 == 0b0001;
    }

    {
        const C: i32 = 0b0011;

        let x = black_box(5i32);
        let _ = x & C == 0b0011;
        let _ = x & C == 0b0100; //~ bad_bit_mask
        let _ = x & 0b0001 == C; //~ bad_bit_mask
    }
}

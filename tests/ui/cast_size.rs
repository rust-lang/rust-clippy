//@stderr-per-bitwidth
//@no-rustfix

#![warn(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]
#![allow(clippy::no_effect, clippy::unnecessary_operation)]

fn main() {
    // Casting from *size
    1isize as i8; //~ cast_possible_truncation
    let x0 = 1isize;
    let x1 = 1usize;
    x0 as f64; //~ cast_precision_loss
    x1 as f64; //~ cast_precision_loss
    x0 as f32; //~ cast_precision_loss
    x1 as f32; //~ cast_precision_loss
    1isize as i32; //~ cast_possible_truncation
    1isize as u32; //~ cast_possible_truncation
    1usize as u32; //~ cast_possible_truncation
    1usize as i32;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap
    1i64 as isize; //~ cast_possible_truncation
    1i64 as usize; //~ cast_possible_truncation
    1u64 as isize;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap
    1u64 as usize; //~ cast_possible_truncation
    1u32 as isize; //~ cast_possible_wrap
    1u32 as usize; // Should not trigger any lint
    1i32 as isize; // Neither should this
    1i32 as usize;
    // Big integer literal to float
    999_999_999 as f32; //~ cast_precision_loss
    9_999_999_999_999_999usize as f64; //~ cast_precision_loss
}

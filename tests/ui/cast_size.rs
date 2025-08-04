//@revisions: 32bit 64bit
//@[32bit]ignore-bitwidth: 64
//@[64bit]ignore-bitwidth: 32
//@no-rustfix: only some diagnostics have suggestions

#![warn(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]
#![allow(clippy::no_effect, clippy::unnecessary_operation)]

fn get_value<T>() -> T {
    todo!()
}

fn main() {
    // Casting from *size
    get_value::<isize>() as i8;
    //~^ cast_possible_truncation
    let x0: isize = get_value();
    let x1: usize = get_value();
    // FIXME(f16_f128): enable f16 and f128 conversions once const eval supports them
    // x0 as f16;
    // x1 as f16;
    x0 as f32;
    //~^ cast_precision_loss
    x1 as f32;
    //~^ cast_precision_loss
    x0 as f64;
    //~^ cast_precision_loss
    x1 as f64;
    //~^ cast_precision_loss
    // x0 as f128;
    // x1 as f128;

    get_value::<isize>() as i32;
    //~^ cast_possible_truncation
    get_value::<isize>() as u32;
    //~^ cast_possible_truncation
    //~| cast_sign_loss
    get_value::<usize>() as u32;
    //~^ cast_possible_truncation
    get_value::<usize>() as i32;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap
    get_value::<i64>() as isize;
    //~^ cast_possible_truncation
    get_value::<i64>() as usize;
    //~^ cast_possible_truncation
    //~| cast_sign_loss
    get_value::<u64>() as isize;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap
    get_value::<u64>() as usize;
    //~^ cast_possible_truncation
    get_value::<u32>() as isize;
    //~^ cast_possible_wrap
    get_value::<u32>() as usize; // Should not trigger any lint
    get_value::<i32>() as isize; // Neither should this
    get_value::<i32>() as usize;
    //~^ cast_sign_loss

    // Big integer literal to float
    // 999_999 as f16;
    999_999_999 as f32;
    //~^ cast_precision_loss
    9_999_999_999_999_999usize as f64;
    //~^ cast_precision_loss
    //~[32bit]^^ ERROR: literal out of range for `usize`
    // 999_999_999_999_999_999_999_999_999_999u128 as f128;
}

fn issue15163() {
    const M: usize = 100;
    const N: u16 = M as u16;
    const O: u16 = (M * 1000) as u16;
    //~^ cast_possible_truncation
}

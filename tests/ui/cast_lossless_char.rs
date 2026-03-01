#![allow(clippy::char_lit_as_u8)]
#![warn(clippy::cast_lossless)]

type I32 = i32;
type U32 = u32;

fn main() {
    let _ = '|' as u32;
    //~^ cast_lossless
    let _ = '|' as i32;

    let _ = '|' as u64;
    //~^ cast_lossless
    let _ = '|' as i64;

    let _ = '|' as u128;
    //~^ cast_lossless
    let _ = '|' as i128;

    let _ = '|' as U32;
    //~^ cast_lossless
    let _ = '|' as I32;

    let _ = '|' as usize;
    let _ = '|' as isize;
    let _ = '|' as i8;
    let _ = '|' as u8;
    let _ = '|' as i16;
    let _ = '|' as u16;
}

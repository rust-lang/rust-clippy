#![warn(clippy::cast_lossless)]
#![allow(clippy::char_lit_as_u8)]

type I32 = i32;
type U32 = u32;

fn main() {
    let _ = 'a' as u32;
    //~^ cast_lossless
    let _ = 'a' as i32;
    let _ = 'a' as u64;
    //~^ cast_lossless
    let _ = 'a' as i64;
    let _ = 'a' as U32;
    //~^ cast_lossless
    let _ = 'a' as I32;

    let _ = 'a' as u8;
    let _ = 'a' as i8;
}

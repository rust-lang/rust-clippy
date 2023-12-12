#![warn(clippy::cast_integer)]

fn main() {
    let u = 1usize;
    let _ = u as u64;
    let _ = u64::try_from(u);

    // unsigned
    let u64: u64 = 1;
    let _ = u64 as u128;
    let _ = u128::from(u64);

    let u32: u32 = 1;
    let _ = u32 as u128;
    let _ = u128::from(u32);
    let _ = u32 as u64;
    let _ = u64::from(u32);

    let u16: u16 = 1;
    let _ = u16 as u128;
    let _ = u128::from(u16);
    let _ = u16 as u64;
    let _ = u64::from(u16);
    let _ = u16 as u32;
    let _ = u32::from(u16);

    let u8: u8 = 1;
    let _ = u8 as u128;
    let _ = u128::from(u8);
    let _ = u8 as u64;
    let _ = u64::from(u8);
    let _ = u8 as u32;
    let _ = u32::from(u8);
    let _ = u8 as u16;
    let _ = u16::from(u8);

    // signed

    let isize: isize = 1;
    let _ = isize as i64;
    let _ = i64::try_from(isize);

    let i64: i64 = 1;
    let _ = i64 as i128;
    let _ = i128::from(i64);

    let i32 = 1;
    let _ = i32 as i128;
    let _ = i128::from(i32);
    let _ = i32 as i64;
    let _ = i64::from(i32);

    let i16: i16 = 1;
    let _ = i16 as i128;
    let _ = i128::from(i16);
    let _ = i16 as i64;
    let _ = i64::from(i16);
    let _ = i16 as i32;
    let _ = i32::from(i16);

    let i8: i8 = 1;
    let _ = i8 as i128;
    let _ = i128::from(i8);
    let _ = i8 as i64;
    let _ = i64::from(i8);
    let _ = i8 as i32;
    let _ = i32::from(i8);
    let _ = i8 as i16;
    let _ = i16::from(i8);
}

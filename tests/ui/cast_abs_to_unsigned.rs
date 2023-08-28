#![warn(clippy::cast_abs_to_unsigned)]
#![allow(clippy::uninlined_format_args, unused)]

fn main() {
    let x: i32 = -42;
    let y: u32 = x.abs() as u32;
    //~^ ERROR: casting the result of `i32::abs()` to u32
    //~| NOTE: `-D clippy::cast-abs-to-unsigned` implied by `-D warnings`
    println!("The absolute value of {} is {}", x, y);

    let a: i32 = -3;
    let _: usize = a.abs() as usize;
    //~^ ERROR: casting the result of `i32::abs()` to usize
    let _: usize = a.abs() as _;
    //~^ ERROR: casting the result of `i32::abs()` to usize
    let _ = a.abs() as usize;
    //~^ ERROR: casting the result of `i32::abs()` to usize

    let a: i64 = -3;
    let _ = a.abs() as usize;
    //~^ ERROR: casting the result of `i64::abs()` to usize
    let _ = a.abs() as u8;
    //~^ ERROR: casting the result of `i64::abs()` to u8
    let _ = a.abs() as u16;
    //~^ ERROR: casting the result of `i64::abs()` to u16
    let _ = a.abs() as u32;
    //~^ ERROR: casting the result of `i64::abs()` to u32
    let _ = a.abs() as u64;
    //~^ ERROR: casting the result of `i64::abs()` to u64
    let _ = a.abs() as u128;
    //~^ ERROR: casting the result of `i64::abs()` to u128

    let a: isize = -3;
    let _ = a.abs() as usize;
    //~^ ERROR: casting the result of `isize::abs()` to usize
    let _ = a.abs() as u8;
    //~^ ERROR: casting the result of `isize::abs()` to u8
    let _ = a.abs() as u16;
    //~^ ERROR: casting the result of `isize::abs()` to u16
    let _ = a.abs() as u32;
    //~^ ERROR: casting the result of `isize::abs()` to u32
    let _ = a.abs() as u64;
    //~^ ERROR: casting the result of `isize::abs()` to u64
    let _ = a.abs() as u128;
    //~^ ERROR: casting the result of `isize::abs()` to u128

    let _ = (x as i64 - y as i64).abs() as u32;
    //~^ ERROR: casting the result of `i64::abs()` to u32
}

#[clippy::msrv = "1.50"]
fn msrv_1_50() {
    let x: i32 = 10;
    assert_eq!(10u32, x.abs() as u32);
}

#[clippy::msrv = "1.51"]
fn msrv_1_51() {
    let x: i32 = 10;
    assert_eq!(10u32, x.abs() as u32);
    //~^ ERROR: casting the result of `i32::abs()` to u32
}

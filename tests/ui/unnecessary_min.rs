#![allow(unused)]
#![warn(clippy::unnecessary_min)]

fn main() {
    /*
     */
    const A: i64 = 45;
    const B: i64 = -1;
    const C: i64 = const_fn(B);
    let _ = (A * B).min(B); // Both are constants
    let _ = C.min(B); // Both are constants
    let _ = B.min(C); // Both are constants

    let _ = (-6_i32).min(9); // Both are Literals
    let _ = 9_u32.min(6); // Both are Literals

    /*
    let _ = 6.min(7_u8); // Both are Literals

    let _ = 0.min(7_u8); // unsigned with zero
    let _ = 7.min(0_u32); // unsigned with zero

    let _ = i32::MIN.min(42); // singed MIN
    let _ = 42.min(i32::MIN); // singed MIN

    let _ = i32::MAX.min(42); // singed MAX
    let _ = 42.min(i32::MAX); // singed MAX

    let _ = 0.min(test_usize()); // unsigned with zero and function
    let _ = test_usize().min(0); // unsigned with zero and function

    let _ = i64::MIN.min(test_i64()); // signed with MIN and function
    let _ = test_i64().min(i64::MIN); // signed with MIN and function

    let _ = i64::MAX.min(test_i64()); // signed with MAX and function
    let _ = test_i64().min(i64::MAX); // signed with MAX and function
    */
}
fn test_usize() -> usize {
    42
}
fn test_i64() -> i64 {
    42
}
const fn const_fn(input: i64) -> i64 {
    -2 * input
}

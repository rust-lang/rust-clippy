//@aux-build:proc_macros.rs
#![warn(clippy::bitwise_not_zero)]
#![allow(clippy::eq_op)]

extern crate proc_macros;

macro_rules! local_macro {
    ($($tt:tt)*) => {
        $($tt)*
    }
}

fn main() {
    assert_eq!(!0, i32::MAX);
    //~^ ERROR: usage of the bitwise not `!` on zero

    // this should be OK:
    const ZERO: i16 = 0;
    assert_eq!(!ZERO, i16::MAX);

    // this should be OK:
    let zero: i8 = 0;
    assert_eq!(!zero, i8::MAX);

    assert_eq!(!0i64, i64::MAX);
    //~^ ERROR: usage of the bitwise not `!` on zero

    assert_eq!(!0_u8, u8::MAX);
    //~^ ERROR: usage of the bitwise not `!` on zero

    assert_eq!(!0isize, isize::MAX);
    //~^ ERROR: usage of the bitwise not `!` on zero

    proc_macros::external!(assert_eq!(!0usize, usize::MAX));

    local_macro!(assert_eq!(!0usize, usize::MAX));
    //~^ ERROR: usage of the bitwise not `!` on zero

    assert_eq!(proc_macros::external!(!0u16), u16::MAX);
    assert_eq!(local_macro!(!0u16), u16::MAX);
    //~^ ERROR: usage of the bitwise not `!` on zero

    assert_eq!(!0u64, proc_macros::external!(u64::MAX));
    //~^ ERROR: usage of the bitwise not `!` on zero

    assert_eq!(!local_macro!(0u32), proc_macros::external!(u32::MAX));
    //~^ ERROR: usage of the bitwise not `!` on zero
}

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
    // OK, !0 == -1, not i32::MAX
    assert_eq!(!0, i32::MAX);

    // this should be OK:
    const ZERO: u16 = 0;
    assert_eq!(!ZERO, u16::MAX);

    // this should be OK:
    let zero: u8 = 0;
    assert_eq!(!zero, u8::MAX);

    assert_eq!(!0u64, u64::MAX);
    //~^ bitwise_not_zero

    assert_eq!(!0_u8, u8::MAX);
    //~^ bitwise_not_zero

    assert_eq!(!0usize, usize::MAX);
    //~^ bitwise_not_zero

    proc_macros::external!(assert_eq!(!0usize, usize::MAX));

    local_macro!(assert_eq!(!0usize, usize::MAX));
    //~^ bitwise_not_zero

    assert_eq!(proc_macros::external!(!0u16), u16::MAX);
    assert_eq!(local_macro!(!0u16), u16::MAX);
    //~^ bitwise_not_zero

    assert_eq!(!0u64, proc_macros::external!(u64::MAX));
    //~^ bitwise_not_zero

    assert_eq!(!local_macro!(0u32), proc_macros::external!(u32::MAX));
    //~^ bitwise_not_zero
}

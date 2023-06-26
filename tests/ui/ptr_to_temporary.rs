//@aux-build:proc_macros.rs
#![allow(clippy::borrow_deref_ref, clippy::deref_addrof, clippy::identity_op, unused)]
#![warn(clippy::ptr_to_temporary)]
#![no_main]

#[macro_use]
extern crate proc_macros;

fn bad_1() -> *const i32 {
    &(100 + *&0) as *const i32
}

fn bad_2() -> *const i32 {
    let a = 0i32;
    &(*&a) as *const i32
}

fn bad_3() -> *const i32 {
    let a = 0i32;
    &a as *const i32
}

fn bad_4() -> *const i32 {
    let mut a = 0i32;
    &a as *const i32
}

fn bad_5() -> *const i32 {
    const A: &i32 = &1i32;
    let mut a = 0i32;

    if true {
        return &(*A) as *const i32;
    }
    &a as *const i32
}

fn fine_1() -> *const i32 {
    &100 as *const i32
}

fn fine_2() -> *const i32 {
    const A: &i32 = &0i32;
    A as *const i32
}

fn fine_3() -> *const i32 {
    const A: &i32 = &0i32;
    &(*A) as *const i32
}

external! {
    fn fine_external() -> *const i32 {
        let a = 0i32;
        &a as *const i32
    }
}

with_span! {
    span
    fn fine_proc_macro() -> *const i32 {
        let a = 0i32;
        &a as *const i32
    }
}

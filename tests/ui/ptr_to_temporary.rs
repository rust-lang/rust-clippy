//@aux-build:proc_macros.rs:proc-macro
#![allow(
    clippy::borrow_deref_ref,
    clippy::deref_addrof,
    clippy::identity_op,
    clippy::unnecessary_cast,
    clippy::unnecessary_operation,
    temporary_cstring_as_ptr,
    unused
)]
#![warn(clippy::ptr_to_temporary)]
#![no_main]

#[macro_use]
extern crate proc_macros;

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::sync::atomic::AtomicBool;

fn bad_1() -> *const i32 {
    // NOTE: `is_const_evaluatable` misses this. This may be a bug in that utils function, and if
    // possible we should switch to it.
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

fn bad_6() {
    let pv = vec![1].as_ptr();
}

fn bad_7() {
    fn helper() -> [i32; 1] {
        [1]
    }

    let pa = helper().as_ptr();
}

fn bad_8() {
    let pc = Cell::new("oops ub").as_ptr();
}

fn bad_9() {
    let prc = RefCell::new("oops more ub").as_ptr();
}

fn bad_10() {
    // Our lint and `temporary_cstring_as_ptr` both catch this. Maybe we can deprecate that one?
    let pcstr = unsafe { CString::new(vec![]).unwrap().as_ptr() };
}

fn bad_11() {
    let pab = unsafe { AtomicBool::new(true).as_ptr() };
}

fn bad_12() {
    let ps = vec![1].as_slice().as_ptr();
}

fn bad_13() {
    let ps = <[i32]>::as_ptr(Vec::as_slice(&vec![1]));
}

fn bad_14() {
    CString::new("asd".to_owned()).unwrap().as_c_str().as_ptr();
}

fn bad_15() {
    let pv = (*vec![12].into_boxed_slice()).as_ptr();
}

fn bad_16() {
    let pv = vec![12].into_boxed_slice().as_ptr();
}

fn bad_17() {
    [1u8, 2].as_mut_ptr();
}

// TODO: We need more tests here...

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

fn fine_4() {
    let pa = ([1],).0.as_ptr();
}

fn fine_5() {
    fn helper() -> &'static str {
        "i'm not ub"
    }

    let ps = helper().as_ptr();
}

fn fine_6() {
    fn helper<'a>() -> &'a str {
        "i'm not ub"
    }

    let ps = helper().as_ptr();
}

fn fine_7() {
    fn helper() -> &'static [i32; 1] {
        &[1]
    }

    let pa = helper().as_ptr();
}

fn fine_8() {
    let ps = "a".as_ptr();
}

fn fine_9() {
    let pcstr = CString::new("asd".to_owned()).unwrap();
    // Not UB, as `pcstr` is not a temporary
    pcstr.as_c_str().as_ptr();
}

fn fine_10() {
    let pcstr = CString::new("asd".to_owned()).unwrap();
    // Not UB, as `pcstr` is not a temporary
    CString::as_c_str(&pcstr).as_ptr();
}

// Should still lint

external! {
    fn bad_external() -> *const i32 {
        let a = 0i32;
        &a as *const i32
    }
    fn bad_external_2() {
        let ps = <[i32]>::as_ptr(Vec::as_slice(&vec![1]));
    }
}

with_span! {
    span
    fn bad_proc_macro() -> *const i32 {
        let a = 0i32;
        &a as *const i32
    }
    fn bad_proc_macro_2() {
        let ps = <[i32]>::as_ptr(Vec::as_slice(&vec![1]));
    }
}

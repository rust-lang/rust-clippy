//@aux-build:unnecessary_unwrap_unchecked_helper.rs
//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::useless_vec, unused)]
#![warn(clippy::unnecessary_unwrap_unchecked)]

#[macro_use]
extern crate proc_macros;

use unnecessary_unwrap_unchecked_helper::lol;

mod b {
    pub fn test_fn() -> Option<u32> {
        Some(0)
    }

    pub fn test_fn_unchecked() -> u32 {
        0
    }
}

fn test_fn() -> Option<u32> {
    Some(0)
}

fn test_fn_unchecked() -> u32 {
    0
}

struct A;

impl A {
    fn a(&self) -> Option<u32> {
        Some(0)
    }

    fn a_unchecked(&self) -> u32 {
        0
    }

    fn an_assoc_fn() -> Option<u32> {
        Some(0)
    }

    fn an_assoc_fn_unchecked() -> u32 {
        0
    }
}

fn main() {
    let string_slice = unsafe { std::str::from_utf8(&[]).unwrap_unchecked() };
    let a = unsafe { A::a(&A).unwrap_unchecked() };
    let a = unsafe {
        let a = A;
        a.a().unwrap_unchecked()
    };
    let an_assoc_fn = unsafe { A::an_assoc_fn().unwrap_unchecked() };
    let extern_fn = unsafe { lol().unwrap_unchecked() };
    let local_fn = unsafe { b::test_fn().unwrap_unchecked() };
    macro_rules! local {
        () => {{
            unsafe { ::std::str::from_utf8(&[]).unwrap_unchecked() };
        }};
    }
    local!();
    external! {
        unsafe { std::str::from_utf8(&[]).unwrap_unchecked() };
    }
    with_span! {
        span
        unsafe { std::str::from_utf8(&[]).unwrap_unchecked() };
    }
}

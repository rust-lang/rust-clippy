//@aux-build:proc_macro_derive.rs

#![warn(clippy::std_instead_of_core)]
#![allow(unused_imports)]

extern crate alloc;

#[macro_use]
extern crate proc_macro_derive;

#[warn(clippy::std_instead_of_core)]
fn std_instead_of_core() {
    // Regular import
    use std::hash::Hasher;
    //~^ std_instead_of_core
    // Absolute path
    use ::std::hash::Hash;
    //~^ std_instead_of_core
    // Don't lint on `env` macro
    use std::env;

    // Multiple imports
    use std::fmt::{Debug, Result};
    //~^ std_instead_of_core

    // Multiple imports multiline
    #[rustfmt::skip]
    use std::{
    //~^ std_instead_of_core
        fmt::Write as _,
        ptr::read_unaligned,
    };

    // Function calls
    let ptr = std::ptr::null::<u32>();
    //~^ std_instead_of_core
    let ptr_mut = ::std::ptr::null_mut::<usize>();
    //~^ std_instead_of_core

    // Types
    let cell = std::cell::Cell::new(8u32);
    //~^ std_instead_of_core
    let cell_absolute = ::std::cell::Cell::new(8u32);
    //~^ std_instead_of_core

    let _ = std::env!("PATH");

    use std::error::Error;
    //~^ std_instead_of_core

    // lint items re-exported from private modules, `core::iter::traits::iterator::Iterator`
    use std::iter::Iterator;
    //~^ std_instead_of_core
}

#[warn(clippy::std_instead_of_alloc)]
fn std_instead_of_alloc() {
    // Only lint once.
    use std::vec;
    //~^ std_instead_of_alloc
    use std::vec::Vec;
    //~^ std_instead_of_core
}

#[warn(clippy::alloc_instead_of_core)]
fn alloc_instead_of_core() {
    use alloc::slice::from_ref;
    //~^ alloc_instead_of_core
}

mod std_in_proc_macro_derive {
    #[warn(clippy::alloc_instead_of_core)]
    #[allow(unused)]
    #[derive(ImplStructWithStdDisplay)]
    struct B {}
}

// Some intrinsics are usable on stable but live in an unstable module, but should still suggest
// replacing std -> core
fn intrinsic(a: *mut u8, b: *mut u8) {
    unsafe {
        std::intrinsics::copy(a, b, 1);
        //~^ std_instead_of_core
        //~^ std_instead_of_core
    }
}

#[clippy::msrv = "1.76"]
fn msrv_1_76(_: std::net::IpAddr) {}

#[clippy::msrv = "1.77"]
fn msrv_1_77(_: std::net::IpAddr) {}
//~^ std_instead_of_core

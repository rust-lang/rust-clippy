//@aux-build:wildcard_imports_helper.rs

#![feature(test)]
#![warn(clippy::std_wildcard_imports)]

extern crate alloc;
extern crate proc_macro;
extern crate test;
extern crate wildcard_imports_helper;

use alloc::boxed::*;
//~^ std_wildcard_imports
use core::cell::*;
//~^ std_wildcard_imports
use proc_macro::*;
//~^ std_wildcard_imports
use std::any::*;
//~^ std_wildcard_imports
use std::mem::{swap, *};
//~^ std_wildcard_imports
use test::bench::*;
//~^ std_wildcard_imports

use crate::fn_mod::*;

use wildcard_imports_helper::*;

use std::io::prelude::*;
use wildcard_imports_helper::extern_prelude::v1::*;
use wildcard_imports_helper::prelude::v1::*;

mod fn_mod {
    pub fn foo() {}
}

mod super_imports {
    use super::*;

    fn test_super() {
        fn_mod::foo();
    }
}

fn main() {
    foo();

    let _ = Box::new(()); // imported from alloc::boxed module
    let _ = Cell::new(()); // imported from core::cell module
    let _ = is_available(); // imported from proc_macro crate
    let _ = type_name::<i32>(); // imported from std::any module
    let _ = align_of::<i32>(); // imported from std::mem module
    black_box(()); // imported from test crate
}

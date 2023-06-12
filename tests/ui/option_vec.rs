//@aux-build:proc_macros.rs
#![allow(unused)]
#![warn(clippy::option_vec)]
#![no_main]

#[macro_use]
extern crate proc_macros;

use std::ptr::NonNull;

// Lint
struct A(std::option::Option<Vec<u16>>);
struct B(Option<Vec<u16>>);
// Do not lint
struct C(Vec<Option<Vec<u16>>>);
struct D(NonNull<Option<Vec<u16>>>);
pub struct E(std::option::Option<Vec<u16>>);
pub struct F(Option<Vec<u16>>);

external! {
    struct G(Option<Vec<u16>>);
}
with_span! {
    span
    struct H(Option<Vec<u16>>);
}

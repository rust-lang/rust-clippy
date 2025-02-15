//@aux-build:proc_macros.rs
#![deny(clippy::empty_structs_with_brackets)]
#![allow(dead_code)]

extern crate proc_macros;
use proc_macros::{external, with_span};

pub struct MyEmptyStruct {} //~ empty_structs_with_brackets
struct MyEmptyTupleStruct(); //~ empty_structs_with_brackets

#[rustfmt::skip]
struct WithWhitespace {  } //~ empty_structs_with_brackets

struct WithComment {
    // Some long explanation here
}

struct MyCfgStruct {
    #[cfg(feature = "thisisneverenabled")]
    field: u8,
}

struct MyCfgTupleStruct(#[cfg(feature = "thisisneverenabled")] u8);

struct MyStruct {
    field: u8,
}
struct MyTupleStruct(usize, String);
struct MySingleTupleStruct(usize);
struct MyUnitLikeStruct;

external! {
    struct External {}
}

with_span! {
    span
    struct ProcMacro {}
}

macro_rules! m {
    ($($name:ident: $ty:ty),*) => {
        struct Macro { $($name: $ty),* }
    }
}
m! {}

fn main() {}

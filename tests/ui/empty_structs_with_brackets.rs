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

mod used {
    // Check if the constructor is passed as an argument.
    struct S1();
    struct S2(); //~ empty_structs_with_brackets
    fn f1(x: i32) -> Option<S1> {
        (x == 1).then(S1)
    }

    // Check that call sites are fixed.
    struct S3(); //~ empty_structs_with_brackets
    struct S4();
    fn f2() {
        let _ = S3();
        let _ = S4;
    }
}

fn main() {}

#[rustfmt::skip]
mod issue15349 {
    struct S1<const N: usize> {} //~ empty_structs_with_brackets
    struct S2<const N: usize> where usize: Copy {} //~ empty_structs_with_brackets
    struct S3 where {} //~ empty_structs_with_brackets
    struct S4<const N: usize>(); //~ empty_structs_with_brackets
    struct S5<const N: usize>() where usize: Copy; //~ empty_structs_with_brackets
    struct S6() where usize: Copy; //~ empty_structs_with_brackets
}

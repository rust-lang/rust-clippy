//@aux-build:proc_macros.rs
#![deny(clippy::empty_enum_variants_with_brackets)]
#![allow(dead_code)]

extern crate proc_macros;
use proc_macros::{external, with_span};

pub enum PublicTestEnum {
    NonEmptyBraces { x: i32, y: i32 },
    NonEmptyParentheses(i32, i32),
    EmptyBraces {},     //~ empty_enum_variants_with_brackets
    EmptyParentheses(), //~ empty_enum_variants_with_brackets
}

enum TestEnum {
    NonEmptyBraces {
        x: i32,
        y: i32,
    },
    NonEmptyParentheses(i32, i32),
    EmptyBraces {},     //~ empty_enum_variants_with_brackets
    EmptyParentheses(), //~ empty_enum_variants_with_brackets
    AnotherEnum,
    #[rustfmt::skip]
    WithWhitespace {  }, //~ empty_enum_variants_with_brackets
    WithComment {
        // Some long explanation here
    },
}

enum TestEnumWithFeatures {
    NonEmptyBraces {
        #[cfg(feature = "thisisneverenabled")]
        x: i32,
    },
    NonEmptyParentheses(#[cfg(feature = "thisisneverenabled")] i32),
}

external! {
    enum External {
        Foo {},
    }
}

with_span! {
    span
    enum ProcMacro {
        Foo(),
    }
}

macro_rules! m {
    ($($ty:ty),*) => {
        enum Macro {
            Foo($($ty),*),
        }
    }
}
m! {}

fn main() {}

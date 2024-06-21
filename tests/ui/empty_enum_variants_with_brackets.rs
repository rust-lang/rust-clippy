#![warn(clippy::empty_enum_variants_with_brackets)]
#![allow(dead_code)]

pub enum PublicTestEnum {
    // No error as this is a reachable enum
    NonEmptyBraces { x: i32, y: i32 },
    NonEmptyParentheses(i32, i32),
    EmptyBraces {},
    EmptyParentheses(),
}

enum TestEnum {
    NonEmptyBraces { x: i32, y: i32 }, // No error
    NonEmptyParentheses(i32, i32),     // No error
    EmptyBraces {},                    //~ ERROR: enum variant has empty brackets
    EmptyParentheses(),                //~ ERROR: enum variant has empty brackets
    AnotherEnum,                       // No error
}

enum EvenOdd {
    // Used as a function
    Even(),
    Odd(),
    // Not used as a function
    Unknown(), //~ ERROR: enum variant has empty brackets
}

fn even_odd(x: i32) -> EvenOdd {
    (x % 2 == 0).then(EvenOdd::Even).unwrap_or_else(EvenOdd::Odd)
}

enum TestEnumWithFeatures {
    NonEmptyBraces {
        #[cfg(feature = "thisisneverenabled")]
        x: i32,
    }, // No error
    NonEmptyParentheses(#[cfg(feature = "thisisneverenabled")] i32), // No error
}

fn main() {}

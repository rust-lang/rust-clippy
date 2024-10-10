//@aux-build:non-exhaustive-enum.rs
#![allow(clippy::manual_unwrap_or_default, clippy::manual_unwrap_or)]
#![warn(clippy::unneeded_struct_pattern)]

extern crate non_exhaustive_enum;
use non_exhaustive_enum::*;

fn main() {
    match Some(114514) {
        Some(v) => v,
        None {} => 0,
    };

    match Some(1919810) {
        Some(v) => v,
        None { .. } => 0,
    };

    match Some(123456) {
        Some(v) => v,
        None => 0,
    };

    match Some(Some(123456)) {
        Some(Some(v)) => v,
        Some(None {}) => 0,
        None {} => 0,
    };

    enum Custom {
        HasFields {
            field: i32,
        },
        HasBracketsNoFields {},
        NoBrackets,
        #[non_exhaustive]
        NoBracketsNonExhaustive,
        Init,
    };

    match Custom::Init {
        Custom::HasFields { field: value } => value, // Should be ignored
        Custom::HasBracketsNoFields {} => 0,         // Should be ignored
        Custom::NoBrackets {} => 0,                  // Should be fixed
        Custom::NoBracketsNonExhaustive {} => 0,     // Should be fixed
        _ => 0,
    };

    match Custom::Init {
        Custom::HasFields { field: value } => value, // Should be ignored
        Custom::HasBracketsNoFields { .. } => 0,     // Should be ignored
        Custom::NoBrackets { .. } => 0,              // Should be fixed
        Custom::NoBracketsNonExhaustive { .. } => 0, // Should be fixed
        _ => 0,
    };

    match Custom::Init {
        Custom::NoBrackets {} if true => 0, // Should be fixed
        _ => 0,
    };

    match Custom::Init {
        Custom::NoBrackets {} | Custom::NoBracketsNonExhaustive {} => 0, // Should be fixed
        _ => 0,
    };
}

fn external_crate() {
    use ExtNonExhaustiveVariant::*;

    match ExhaustiveUnit {
        // Expected
        ExhaustiveUnit => 0,
        _ => 0,
    };

    match ExhaustiveUnit {
        // Exhaustive variant, should be fixed
        ExhaustiveUnit { .. } => 0,
        _ => 0,
    };

    match ExhaustiveUnit {
        // Exhaustive variant, should be fixed
        ExhaustiveUnit {} => 0,
        _ => 0,
    };

    match ExhaustiveUnit {
        ExhaustiveUnit => 0,
        // vvvvv Non-exhaustive variants, should all be ignored
        Unit { .. } => 0,
        Tuple { 0: field, .. } => field,
        StructNoField { .. } => 0,
        Struct { field, .. } => field,
        _ => 0,
    };
}

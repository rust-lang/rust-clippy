#![allow(unused)]
// Enable lint to check relative paths in macro definitions
#![warn(clippy::relative_path_in_macro_definition)]

// Macro with relative path to core (triggers lint)
#[macro_export]
macro_rules! relative_path_macro {
    ($condition:expr) => {
        const _: () = core::assert!($condition);
        //~^ relative_path_in_macro_definition
    };
}

// Macro with relative path to std (triggers lint)
#[macro_export]
macro_rules! relative_std {
    () => {
        let _ = std::mem::size_of::<i32>();
        //~^ relative_path_in_macro_definition
    };
}

// Macro with absolute path to core (no lint)
#[macro_export]
macro_rules! absolute_path_assert {
    ($condition:expr) => {
        const _: () = ::core::assert!($condition);
    };
}

// Macro with absolute path to std (no lint)
#[macro_export]
macro_rules! absolute_std {
    () => {
        let _ = ::std::mem::size_of::<i32>();
    };
}

// Macro with no path references (no lint)
#[macro_export]
macro_rules! no_path {
    () => {
        let x = 42;
    };
}

// Test all macros
fn main() {
    const X: &[u8] = b"rust";
    relative_path_macro!(X[0] == b'r'); // Test relative core path
    absolute_path_assert!(X[0] == b'r'); // Test absolute core path
    relative_std!(); // Test relative std path
    absolute_std!(); // Test absolute std path
    no_path!(); // Test no path
}

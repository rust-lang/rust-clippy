#![allow(unused)]
#![warn(clippy::relative_path_in_macro_definition)]

#[macro_export]
macro_rules! relative_path_macro {
    ($condition:expr) => {
        const _: () = core::assert!($condition);
        //~^ relative_path_in_macro_definition
    };
}

#[macro_export]
macro_rules! absolute_path_assert {
    ($condition:expr) => {
        const _: () = ::core::assert!($condition);
    };
}

fn main() {
    const X: &[u8] = b"rust";
    relative_path_macro!(X[0] == b'r');
    absolute_path_assert!(X[0] == b'r');
}

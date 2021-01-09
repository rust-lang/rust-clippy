// run-rustfix
#![warn(clippy::into_and_try_into_instead_of_from_and_try_from)]
use std::convert::TryFrom;

fn foo<T>(a: T) where u32: From<T> {}
fn bar<T>(a: T) where u32: TryFrom<T> {}

fn main() {
    // test code goes here
}

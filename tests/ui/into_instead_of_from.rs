// run-rustfix
#![warn(clippy::into_instead_of_from)]
use std::convert::TryFrom;

fn foo<T>(a: T) where u32: From<T> {}
fn bar<T>(a: T) where u32: TryFrom<T> {}

fn main() {
}

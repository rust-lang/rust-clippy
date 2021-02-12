// run-rustfix
#![warn(clippy::from_instead_of_into)]
use std::convert::TryFrom;

fn foo<T>(a: T) where u32: From<T> {}
fn bar<T>(a: T) where u32: TryFrom<T> {}

fn main() {
}

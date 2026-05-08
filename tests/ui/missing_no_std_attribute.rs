#![warn(clippy::missing_no_std_attribute)]

pub const FOO: Foo = Foo; //~missing_no_std_attribute

mod foo {
    pub struct Foo;
}

pub use foo::Foo;

use core::fmt::Debug as _;

fn main() {
    // test code goes here
}

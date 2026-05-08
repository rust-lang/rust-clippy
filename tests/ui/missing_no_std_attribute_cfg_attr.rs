//@check-pass
#![warn(clippy::missing_no_std_attribute)]
#![cfg_attr(false, no_std)]

pub const FOO: Foo = Foo;

mod foo {
    pub struct Foo;
}

pub use foo::Foo;

use core::fmt::Debug as _;

fn main() {
    // test code goes here
}

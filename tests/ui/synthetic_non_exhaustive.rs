#![allow(unused)]
#![warn(clippy::synthetic_non_exhaustive)]

pub enum Foo1 {
    A,
    B,
    C,
}

pub enum Foo2 {
    A,
    B,
    #[doc(hidden)]
    C,
}

#[non_exhaustive]
pub enum Foo3 {
    A,
    B,
    #[doc(hidden)]
    C,
}

fn main() {
    // test code goes here
}

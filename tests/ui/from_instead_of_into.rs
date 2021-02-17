// run-rustfix
#![allow(unused_imports)]
#![warn(clippy::from_instead_of_into)]
use std::convert::TryFrom;

fn foo<T>(a: T)
where
    u32: From<T>,
{
}

fn foo1<T>(a: T)
where u32: Copy + Clone + From<T>,
{
}

fn bar<T>(a: T)
where
    u32: TryFrom<T>,
{
}

fn bar1<T>(a: T)
where
    u32: Copy + Clone + TryFrom<T>,
{
}

fn main() {}

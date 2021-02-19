// run-rustfix
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![warn(clippy::from_instead_of_into)]
use std::convert::TryFrom;
use std::convert::TryInto;

fn foo<T>(a: T)
where
    u32: From<T>,
{
}

fn foo1<T>(a: T)
where
    u32: Copy + Clone + From<T>,
{
}

fn bar<T>(a: T)
where
    u32: TryFrom<T>,
{
}

fn bar1<T>(a: T)
where
    u32: Copy + TryFrom<T> + Clone,
{
}

fn bar2<T>(a: T)
where
    u32: TryFrom<T> + Copy + Clone,
{
}

fn main() {}

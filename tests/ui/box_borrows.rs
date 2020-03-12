#![warn(clippy::all)]
#![allow(clippy::boxed_local, clippy::needless_pass_by_value)]
#![allow(clippy::blacklisted_name)]

pub struct MyStruct {}

pub struct SubT<T> {
    foo: T,
}

pub enum MyEnum {
    One,
    Two,
}

pub fn test<T>(foo: Box<&T>) {}

pub fn test1(foo: Box<&usize>) {}

pub fn test2(foo: Box<&MyStruct>) {}

pub fn test3(foo: Box<&MyEnum>) {}

pub fn test4(foo: Box<&&MyEnum>) {}

pub fn test5(foo: Box<Box<&usize>>) {}

pub fn test6(foo: Box<SubT<&usize>>) {}

fn main() {}

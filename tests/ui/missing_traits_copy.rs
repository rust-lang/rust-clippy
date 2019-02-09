#![feature(untagged_unions)]
#![allow(dead_code)]
#![allow(clippy::all)]
#![warn(clippy::missing_copy_implementations)]

pub enum A {}

#[derive(Clone, Copy)]
pub enum B {}

pub enum C {}

impl Copy for C {}
impl Clone for C {
    fn clone(&self) -> C { *self }
}

pub struct Foo;

#[derive(Clone, Copy)]
pub struct Bar;

pub struct Baz;

impl Copy for Baz {}
impl Clone for Baz {
    fn clone(&self) -> Baz { *self }
}

struct PrivateStruct;

enum PrivateEnum {}

struct GenericType<T>(T);

fn main() {}

#![feature(untagged_unions)]
#![allow(dead_code)]
#![allow(clippy::all)]
#![warn(clippy::missing_debug_implementations)]

pub enum A {}

#[derive(Debug)]
pub enum B {}

pub enum C {}

impl std::fmt::Debug for C {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("C").finish()
    }
}

pub struct Foo;

#[derive(Debug)]
pub struct Bar;

pub struct Baz;

impl std::fmt::Debug for Baz {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Baz").finish()
    }
}

struct PrivateStruct;

enum PrivateEnum {}

struct GenericType<T>(T);

fn main() {}

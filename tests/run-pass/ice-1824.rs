#![feature(plugin)]
#![plugin(clippy)]
#![allow(clippy)]

pub trait A {
    fn b(&self, _c: *const ()) { panic!() }
}

fn main() {}

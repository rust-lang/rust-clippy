#![allow(dead_code)]
#![warn(clippy::ambiguous_method_names)]

fn main() {}

fn ambiguous() {}

trait MyTrait {
    fn ambiguous(&self);
    fn also_ambiguous(&self);
    fn ambiguous_default(&self) {}
}

trait Another {
    fn another(&self);
}

struct A;

impl A {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
    fn ambiguous_default(&self) {}
    fn unambiguous(&self) {}
    fn another(&self) {}
}

impl MyTrait for A {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
}

impl Another for A {
    fn another(&self) {}
}

struct B;

impl B {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
    fn ambiguous_default(&self) {}
    fn another(&self) {}
}

impl MyTrait for B {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
}

impl Another for B {
    fn another(&self) {}
}

struct C;

impl MyTrait for C {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
}

struct D;

impl D {
    fn ambiguous(&self) {}
    fn also_ambiguous(&self) {}
}

struct S<T>(T);

impl S<i32> {
    fn f(&self) {}
}

impl S<u64> {
    fn f(&self) {}
}

#![allow(unused)]
#![warn(clippy::foo_functions)]

struct A;
impl A {
    pub fn fo(&self) {}
    pub fn foo(&self) {}
    //~^ foo_functions
    pub fn food(&self) {}
}

trait B {
    fn fo(&self) {}
    fn foo(&self) {}
    //~^ foo_functions
    fn food(&self) {}
}

fn fo() {}
fn foo() {}
//~^ foo_functions
fn food() {}

fn main() {
    foo();
    let a = A;
    a.foo();
}

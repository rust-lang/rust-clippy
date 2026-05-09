#![allow(unused)]
#![warn(clippy::unordered_methods)]

// Impl methods
struct A;
impl A {
    fn do_nothing() {}

    fn fo(&self) {}

    pub fn foo(&self) {}
    //~^ unordered_methods

    pub fn my_constructor() -> Self {
        //~^ unordered_methods
        A
    }
}

struct B;
impl B {
    pub fn my_constructor() -> Self {
        B
    }

    pub fn my_constructor2() -> Self {
        B
    }
}

struct C;
impl C {
    pub fn foo(&self) {}

    pub fn maybe_constructor() -> Option<Self> {
        //~^ unordered_methods
        Option::Some(C)
    }
}

fn main() {
    let a = A::my_constructor();
    a.foo();
}

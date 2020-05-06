#![warn(clippy::to_string_in_display)]

use std::fmt;

struct A;
impl A {
    fn fmt(&self) {
        self.to_string();
    }
}

trait B {
    fn fmt(&self) {}
}

impl B for A {
    fn fmt(&self) {
        self.to_string();
    }
}

impl fmt::Display for A {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

fn fmt(a: A) {
    a.to_string();
}

fn main() {
    let a = A;
    a.to_string();
    a.fmt();
    fmt(a);
}

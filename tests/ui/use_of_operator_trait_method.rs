use std::ops::Add;

struct A(i32);

struct B(i32);

impl B {
    fn mul(self, other: Self) -> i32 {
        self.0 * other.0
    }
}

impl Add for A {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self (self.0 + other.0)
    }
}

fn main() {
    let a = B(16);
    let b = B(32);
    a.mul(b);

    let a = A(16);
    let b = A(32);
    let _c = a.add(b);

    let a = A(16);
    let b = A(32);
    let _d = a + b;
}
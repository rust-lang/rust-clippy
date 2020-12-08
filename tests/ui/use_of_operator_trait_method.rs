// run-rustfix

#![warn(clippy::use_of_operator_trait_method)]

use std::ops::*;

struct A(i32);
struct B(i32);
struct C(i32);

impl A {
    fn mul(self, other: Self) -> i32 {
        self.0 * other.0
    }
}

impl Add for A {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for B {
    type Output = i32;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Div<B> for A {
    type Output = C;

    fn div(self, other: B) -> Self::Output {
        C(self.0 / other.0)
    }
}

macro_rules! foo {
    () => {
        A(16_i32.mul(17_i32))
    };
}

fn test_primitives() {
    let _ = 16_i32.add(13);
    let _ = 7_i32.sub(19);
    let _ = 15_i32.mul(33);
    let _ = 26_i32.div(7);

    let mut num = 50.0;
    num.sub_assign(4.0);
    num.add_assign(8.0);
    num.mul_assign(2.0);
    num.div_assign(6.0);
    let _ = num.neg();

    let _ = true.not();

    let _ = 0xff_u8.bitand(0xf0).bitor(0x04).bitxor(0x80);
}

fn test_custom_impl() {
    let _ = A(5).add(A(5));
    let _ = B(34).sub(B(21));
    let _ = A(1).div(B(17));
}

fn main() {
    test_primitives();
    test_custom_impl();

    // Don't warn in macros
    foo!();

    // Don't warn because A doesn't impl std::ops::Mul
    let _ = A(16).mul(A(4));
}

#![allow(
    unused,
    dead_code,
    clippy::needless_lifetimes,
    clippy::needless_pass_by_value,
    clippy::needless_arbitrary_self_type
)]
#![warn(clippy::extra_unused_lifetimes)]

fn empty() {}

fn used_lt<'a>(x: &'a u8) {}

fn unused_lt<'a>(x: u8) {}

fn unused_lt_transitive<'a, 'b: 'a>(x: &'b u8) {
    // 'a is useless here since it's not directly bound
}

fn lt_return<'a, 'b: 'a>(x: &'b u8) -> &'a u8 {
    panic!()
}

fn lt_return_only<'a>() -> &'a u8 {
    panic!()
}

fn unused_lt_blergh<'a>(x: Option<Box<dyn Send + 'a>>) {}

trait Foo<'a> {
    fn x(&self, a: &'a u8);
}

impl<'a> Foo<'a> for u8 {
    fn x(&self, a: &'a u8) {}
}

struct Bar;

impl Bar {
    fn x<'a>(&self) {}
}

// test for #489 (used lifetimes in bounds)
pub fn parse<'a, I: Iterator<Item = &'a str>>(_it: &mut I) {
    unimplemented!()
}
pub fn parse2<'a, I>(_it: &mut I)
where
    I: Iterator<Item = &'a str>,
{
    unimplemented!()
}

struct X {
    x: u32,
}

impl X {
    fn self_ref_with_lifetime<'a>(&'a self) {}
    fn explicit_self_with_lifetime<'a>(self: &'a Self) {}
}

// Methods implementing traits must have matching lifetimes
mod issue4291 {
    trait BadTrait {
        fn unused_lt<'a>(x: u8) {}
    }

    impl BadTrait for () {
        fn unused_lt<'a>(_x: u8) {}
    }
}

mod issue6437 {
    pub struct Scalar;

    impl<'a> std::ops::AddAssign<&Scalar> for &mut Scalar {
        fn add_assign(&mut self, _rhs: &Scalar) {
            unimplemented!();
        }
    }

    impl<'b> Scalar {
        pub fn something<'c>() -> Self {
            Self
        }
    }
}

// https://github.com/rust-lang/rust-clippy/pull/8737#pullrequestreview-951268213
mod first_case {
    use serde::de::Visitor;
    pub trait Expected {
        fn fmt(&self, formatter: &mut std::fmt::Formatter);
    }

    impl<'de, T> Expected for T
    where
        T: Visitor<'de>,
    {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) {}
    }
}

// https://github.com/rust-lang/rust-clippy/pull/8737#pullrequestreview-951268213
mod second_case {
    pub trait Source {
        fn hey();
    }

    impl<'a, T: Source + ?Sized + 'a> Source for Box<T> {
        fn hey() {}
    }
}

fn main() {}

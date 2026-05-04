#![warn(clippy::unused_impl)]

trait MyTrait {}

// Should work inside modules
mod bar {
    use crate::MyTrait;

    struct Foo1;
    impl Foo1 {}
    //~^ unused_impl

    struct Foo2<'a>(&'a str);
    // Lifetimes should have no effect
    impl<'a> Foo2<'a> {}
    //~^ unused_impl
}

// Implementations with `#[cfg]` and/or doc comments shouldn't lint

// Doc comment only should lint
struct Bar1;
/// Hello world
impl Bar1 {}

// #[cfg] only should lint
struct Bar2;
#[cfg(test)]
impl Bar2 {}

// Both doc comment and attribute should lint
struct Bar3;
/// Hello world
#[cfg(test)]
impl Bar3 {}

// Different attributes should lint
struct Bar4;
#[doc(hidden)]
impl Bar4 {}
//~^ unused_impl

// Just to make sure, lint check attributes should still work
struct Bar5;
#[expect(clippy::unused_impl)]
impl Bar5 {}

struct Baz;
// Non-empty shouldn't lint
impl Baz {
    fn baz() {}
}
// Trait implementation shouldn't lint
impl MyTrait for Baz {}

macro_rules! generate_impl {
    ($struct:ident) => {
        impl $struct {}
    };
}

struct Qux;
// Macro expansions shouldn't lint
generate_impl!(Qux);

fn main() {}

#![warn(clippy::empty_drop)]
#![allow(unused)]

// should cause an error
struct Foo;

impl Drop for Foo {
//~^ ERROR: empty drop implementation
//~| NOTE: `-D clippy::empty-drop` implied by `-D warnings`
    fn drop(&mut self) {}
}

// shouldn't cause an error
struct Bar;

impl Drop for Bar {
    fn drop(&mut self) {
        println!("dropping bar!");
    }
}

// should error
struct Baz;

impl Drop for Baz {
//~^ ERROR: empty drop implementation
    fn drop(&mut self) {
        {}
    }
}

fn main() {}

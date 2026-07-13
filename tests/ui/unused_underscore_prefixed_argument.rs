#![warn(clippy::unused_underscore_prefixed_argument)]

fn foo(a: i32, _b: i32) {
    //~^ unused_underscore_prefixed_argument
    println!("{a}");
}

pub fn foo2(a: i32, _b: i32) {}

fn main() {}

#![warn(clippy::unused_underscore_prefixed_argument)]

fn foo(a: i32, _b: i32) {
    //~^ unused_underscore_prefixed_argument
    println!("{a}");
}

pub fn foo2(a: i32, _b: i32) {}
// no warnings

pub struct PubStruct;

impl PubStruct {
    fn foo3(a: i32, _b: i32) {
        //~^ unused_underscore_prefixed_argument
        println!("{a}");
    }

    pub fn foo4(a: i32, _b: i32) {}
}

struct PrivStruct;

impl PrivStruct {
    fn foo3(a: i32, _b: i32) {
        //~^ unused_underscore_prefixed_argument
        println!("{a}");
    }

    pub fn foo4(a: i32, _b: i32) {}
    //~^ unused_underscore_prefixed_argument
    // Will still raise lint because this is within a private struct
}

fn main() {}

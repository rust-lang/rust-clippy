#![warn(clippy::unused_underscore_prefixed_argument)]

unsafe extern "C" {
    fn abcd(_a: i32);
}

struct S {
    some_func: unsafe fn(a: i32, u32, u32)
}

unsafe fn shared_some_func(a: i32, _b: u32, _c: u32) {
    println!("{a}");
}

static SHARED_S: S = S { some_func: shared_some_func };

fn foo(a: i32, _b: i32) {
    //~^ unused_underscore_prefixed_argument
    println!("{a}");
}

pub fn foo2(a: i32, _b: i32) {}
// no warning

fn main() {}

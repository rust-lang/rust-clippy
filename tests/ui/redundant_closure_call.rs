#![warn(clippy::redundant_closure_call)]
#![allow(deprecated)] // for `try` macro

fn main() {
    let a = (|| 42)();

    let mut i = 1;
    let mut k = (|m| m + 1)(i);

    k = (|a, b| a * b)(1, 5);

    let closure = || 32;
    i = closure();

    let closure = |i| i + 1;
    i = closure(3);

    i = closure(4);

    #[allow(clippy::needless_return)]
    (|| return 2)();
    (|| -> Option<i32> { None? })();
    (|| -> Result<i32, i32> { r#try!(Err(2)) })();
}

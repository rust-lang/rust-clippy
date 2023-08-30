#![allow(unused, clippy::unnecessary_literal_unwrap)]

struct MyTypeNonDebug;

#[derive(Debug)]
struct MyTypeDebug;

fn main() {
    let test_debug: Result<MyTypeDebug, u32> = Ok(MyTypeDebug);
    test_debug.err().expect("Testing debug type");
    //~^ ERROR: called `.err().expect()` on a `Result` value
    //~| NOTE: `-D clippy::err-expect` implied by `-D warnings`

    let test_non_debug: Result<MyTypeNonDebug, u32> = Ok(MyTypeNonDebug);
    test_non_debug.err().expect("Testing non debug type");
}

#[clippy::msrv = "1.16"]
fn msrv_1_16() {
    let x: Result<u32, &str> = Ok(16);
    x.err().expect("16");
}

#[clippy::msrv = "1.17"]
fn msrv_1_17() {
    let x: Result<u32, &str> = Ok(17);
    x.err().expect("17");
    //~^ ERROR: called `.err().expect()` on a `Result` value
}

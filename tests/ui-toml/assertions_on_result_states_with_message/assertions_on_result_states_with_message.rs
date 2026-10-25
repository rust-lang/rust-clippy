#![warn(clippy::assertions_on_result_states)]
#![allow(clippy::unnecessary_literal_unwrap)]

#[derive(Debug)]
struct DebugFoo;

struct Foo;

fn main() {
    let r: Result<DebugFoo, DebugFoo> = Ok(DebugFoo);
    assert!(r.is_ok());
    //~^ assertions_on_result_states

    let r: Result<DebugFoo, DebugFoo> = Ok(DebugFoo);
    assert!(r.is_ok(), "should be ok");
    //~^ assertions_on_result_states

    let r: Result<DebugFoo, DebugFoo> = Err(DebugFoo);
    assert!(r.is_err(), "should be err");
    //~^ assertions_on_result_states

    let sql = "SELECT";
    let r: Result<DebugFoo, DebugFoo> = Err(DebugFoo);
    assert!(r.is_err(), "{sql} should not parse");
    //~^ assertions_on_result_states

    let r: Result<DebugFoo, DebugFoo> = Ok(DebugFoo);
    assert!(r.is_ok(), "{}", sql);
    //~^ assertions_on_result_states

    // the unexpected value can't be printed
    let r: Result<DebugFoo, Foo> = Ok(DebugFoo);
    assert!(r.is_ok(), "should be ok");
}

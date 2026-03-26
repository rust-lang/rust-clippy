#![warn(clippy::empty_expect)]
#![allow(clippy::unnecessary_literal_unwrap)]

fn main() {
    let x: Option<i32> = Some(1);
    let _ = x.expect(""); // should warn
    //~^ empty_expect
    let _ = x.expect("valid value"); // should not warn

    let v: Result<i32, &str> = Ok(1);
    let _ = v.expect(""); // should warn
    //~^ empty_expect
    let _ = v.expect("parsing succeeds"); // should not warn
}

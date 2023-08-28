#![allow(clippy::unnecessary_operation)]
#![warn(clippy::bytes_nth)]

fn main() {
    let s = String::from("String");
    let _ = s.bytes().nth(3);
    //~^ ERROR: called `.bytes().nth()` on a `String`
    //~| NOTE: `-D clippy::bytes-nth` implied by `-D warnings`
    let _ = &s.bytes().nth(3).unwrap();
    //~^ ERROR: called `.bytes().nth().unwrap()` on a `String`
    let _ = s[..].bytes().nth(3);
    //~^ ERROR: called `.bytes().nth()` on a `str`
}

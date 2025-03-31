//@aux-build:macro_rules.rs
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::allow_attributes)]

use std::env::var;

pub fn f() -> Result<(), &'static str> {
    //~^ missing_errors_doc
    match "test" {
        "hello" => Ok(()),
        _ => Err("Oh no"),
    }
}

pub fn g() {
    //~^ missing_panics_doc
    panic!();
}

#[expect(clippy::missing_panics_doc)]
#[expect(clippy::missing_errors_doc)]
pub fn main() -> Result<(), &'static str> {
    let val = var("Hello").unwrap();
    match val.as_str() {
        "hello" => Ok(()),
        _ => Err("Oh no"),
    }
}

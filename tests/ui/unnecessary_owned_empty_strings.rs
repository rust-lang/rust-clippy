#![warn(clippy::unnecessary_owned_empty_strings)]

fn ref_str_argument(_value: &str) {}

#[allow(clippy::ptr_arg)]
fn ref_string_argument(_value: &String) {}

fn main() {
    // should be linted
    ref_str_argument(&String::new());
    //~^ ERROR: usage of `&String::new()` for a function expecting a `&str` argument
    //~| NOTE: `-D clippy::unnecessary-owned-empty-strings` implied by `-D warnings`

    // should be linted
    #[allow(clippy::manual_string_new)]
    ref_str_argument(&String::from(""));
    //~^ ERROR: usage of `&String::from("")` for a function expecting a `&str` argument

    // should not be linted
    ref_str_argument("");

    // should not be linted
    ref_string_argument(&String::new());
}

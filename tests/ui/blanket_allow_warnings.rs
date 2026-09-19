// HACK?: `deny` instead of `warn`
#![deny(clippy::blanket_allow_warnings)]
#![allow(warnings)]
//~^ blanket_allow_warnings
#![expect(warnings)]
//~^ blanket_allow_warnings
#![allow(
    innocuous,
    warnings,
//~^ blanket_allow_warnings
    alright
)]
#![expect(
    acceptable,
    reasonable,
    warnings
//~^ blanket_allow_warnings
)]
#![allow(warnings, reason = "shrug")]
//~^ blanket_allow_warnings
#![expect(warnings, reason = "shrug")]
//~^ blanket_allow_warnings

#[allow(warnings)]
//~^ blanket_allow_warnings
fn on_item(x: u32) {}

fn ok() {
    #![deny(warnings)]
    #![forbid(warnings)]
}

fn specific() {
    #[allow(unused)]
    let x = 43;
}

#[recursion_limit = "4"]
fn other_attr() {}

fn main() {}

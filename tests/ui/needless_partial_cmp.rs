#![warn(clippy::needless_partial_cmp)]
#![allow(clippy::needless_ifs)]
//@no-rustfix
use std::cmp::Ordering;

#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct HasOrd(u32);

#[derive(PartialOrd, PartialEq)]
struct NoOrd(i32);
fn main() {
    let a = HasOrd(6);
    let b = HasOrd(87);
    let c = NoOrd(4);
    let d = NoOrd(5);

    let _ = a.partial_cmp(&b);
    //~^ needless_partial_cmp

    if a.partial_cmp(&b).is_some() {}
    //~^ needless_partial_cmp

    if c.partial_cmp(&d).is_some() {}
}

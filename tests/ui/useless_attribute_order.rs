//@no-rustfix
#![warn(clippy::useless_attribute)]
#![feature(rustc_private)]

// A lint attribute that is permitted on the item only excuses itself, never its siblings. The same
// pair of attributes must therefore be reported no matter which order it appears in, so the
// annotations below are a symmetry assertion rather than a snapshot.

#[allow(unused_imports)]
#[allow(clippy::almost_swapped)]
//~^ useless_attribute
use std::cmp::Ordering as OrderingA;

#[allow(clippy::almost_swapped)]
//~^ useless_attribute
#[allow(unused_imports)]
use std::cmp::Ordering as OrderingB;

#[allow(unused_extern_crates)]
#[allow(clippy::almost_swapped)]
//~^ useless_attribute
extern crate regex as regex_a;

#[allow(clippy::almost_swapped)]
//~^ useless_attribute
#[allow(unused_extern_crates)]
extern crate regex as regex_b;

fn main() {}

//@aux-build:fake_no_std.rs
//@aux-build:std_dependent_crate1.rs
#![allow(unused)]
#![warn(clippy::no_std_crate_std_dependent)]
#![no_std]
#![no_main]

// not actually no_std
extern crate fake_no_std;
extern crate std_dependent_crate1;

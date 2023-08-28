#![warn(clippy::mismatched_target_os)]
#![allow(unused)]

#[cfg(hermit)]
//~^ ERROR: operating system used in target family position
//~| NOTE: `-D clippy::mismatched-target-os` implied by `-D warnings`
fn hermit() {}

#[cfg(wasi)]
//~^ ERROR: operating system used in target family position
fn wasi() {}

#[cfg(none)]
//~^ ERROR: operating system used in target family position
fn none() {}

// list with conditions
#[cfg(all(not(windows), wasi))]
//~^ ERROR: operating system used in target family position
fn list() {}

// windows is a valid target family, should be ignored
#[cfg(windows)]
fn windows() {}

// correct use, should be ignored
#[cfg(target_os = "hermit")]
fn correct() {}

fn main() {}

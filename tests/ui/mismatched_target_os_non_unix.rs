#![warn(clippy::mismatched_target_os)]
#![allow(unused)]

#[cfg(hermit)] //~ mismatched_target_os
fn hermit() {}

#[cfg(wasi)] //~ mismatched_target_os
fn wasi() {}

#[cfg(none)] //~ mismatched_target_os
fn none() {}

// list with conditions
#[cfg(all(not(windows), wasi))] //~ mismatched_target_os
fn list() {}

// windows is a valid target family, should be ignored
#[cfg(windows)]
fn windows() {}

// correct use, should be ignored
#[cfg(target_os = "hermit")]
fn correct() {}

fn main() {}

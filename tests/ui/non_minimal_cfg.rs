#![allow(unused)]

#[cfg(all(windows))]
//~^ ERROR: unneeded sub `cfg` when there is only one condition
//~| NOTE: `-D clippy::non-minimal-cfg` implied by `-D warnings`
fn hermit() {}

#[cfg(any(windows))]
//~^ ERROR: unneeded sub `cfg` when there is only one condition
fn wasi() {}

#[cfg(all(any(unix), all(not(windows))))]
//~^ ERROR: unneeded sub `cfg` when there is only one condition
//~| ERROR: unneeded sub `cfg` when there is only one condition
fn the_end() {}

#[cfg(any())]
fn any() {}

fn main() {}

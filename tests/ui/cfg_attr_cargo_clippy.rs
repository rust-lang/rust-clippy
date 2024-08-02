#![warn(clippy::deprecated_clippy_cfg_attr)]
#![allow(clippy::non_minimal_cfg)]
#![cfg_attr(feature = "cargo-clippy", doc = "a")] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr

#[cfg_attr(feature = "cargo-clippy", derive(Debug))] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
#[cfg_attr(not(feature = "cargo-clippy"), derive(Debug))] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
#[cfg(feature = "cargo-clippy")] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
#[cfg(not(feature = "cargo-clippy"))] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
#[cfg(any(feature = "cargo-clippy"))] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
#[cfg(all(feature = "cargo-clippy"))] //~ ERROR: `feature = "cargo-clippy"` was
//~^ deprecated_clippy_cfg_attr
pub struct Bar;

fn main() {}

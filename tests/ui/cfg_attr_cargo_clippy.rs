#![warn(clippy::deprecated_clippy_cfg_attr)]
#![allow(clippy::non_minimal_cfg)]
#![cfg_attr(feature = "cargo-clippy", doc = "a")]
//~^ deprecated_clippy_cfg_attr

#[cfg_attr(feature = "cargo-clippy", derive(Debug))]
//~^ deprecated_clippy_cfg_attr
#[cfg_attr(not(feature = "cargo-clippy"), derive(Debug))]
//~^ deprecated_clippy_cfg_attr
struct Attrs;

#[cfg(not(feature = "cargo-clippy"))]
//~^ deprecated_clippy_cfg_attr
struct Not;

#[cfg(feature = "cargo-clippy")]
//~^ deprecated_clippy_cfg_attr
struct Stripped;

#[cfg(any(feature = "cargo-clippy"))]
//~^ deprecated_clippy_cfg_attr
struct StrippedAny;

#[cfg(all(feature = "cargo-clippy"))]
//~^ deprecated_clippy_cfg_attr
struct StrippedAll;

fn main() {}

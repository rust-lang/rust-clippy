//@ check-pass

#![warn(clippy::missing_inline_in_public_items)]
#![allow(clippy::disallowed_names)]

pub fn foo() {}

fn main() {}

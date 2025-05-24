//@aux-build:proc_macros.rs
#![warn(clippy::mem_replace_with_default)]
#![allow(clippy::relative_path_in_macro_definition)]

extern crate proc_macros;
use proc_macros::{external, inline_macros};

#[inline_macros]
fn main() {
    let s = &mut String::from("foo");
    let _ = inline!(std::mem::replace($s, Default::default()));
    //~^ mem_replace_with_default
    let _ = external!(std::mem::replace($s, Default::default()));
}

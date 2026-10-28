//@check-pass
//@aux-build:../auxiliary/proc_macro_attr.rs

// A proc macro generates `#[doc = "..."]` text itself, but gives the string literal the span of a
// user-written identifier. The user can't edit that text, so `doc_markdown` must not lint it, even
// though the flagged word also appears verbatim in the source at that span.

#![warn(clippy::doc_markdown)]

extern crate proc_macro_attr;
use proc_macro_attr::doc_args_with_ident_span;

/// Does a thing.
#[doc_args_with_ident_span]
pub fn single_arg(border_radius: u32) -> u32 {
    border_radius
}

// Several generated `#[doc]` attributes, each with a different identifier span.
/// Does another thing.
#[doc_args_with_ident_span]
pub fn several_args(border_radius: u32, background_color: u32) -> u32 {
    border_radius + background_color
}

// raw ident, not raw string
/// Does a raw thing.
#[doc_args_with_ident_span]
pub fn raw_ident_arg(r#border_radius: u32) -> u32 {
    r#border_radius
}

fn main() {}

#![feature(array_try_map, array_windows, if_let_guard, io_error_more, rustc_private)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications
)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    rustc::diagnostic_outside_of_impl,
    rustc::untranslatable_diagnostic
)]
#![deny(clippy::derive_deserialize_allowing_unknown)]

extern crate rustc_attr_parsing;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[macro_use]
mod de;
mod conf;
mod metadata;
pub mod types;

pub use conf::{Conf, sanitize_explanation};
pub use metadata::ConfMetadata;

#![feature(rustc_private, array_windows, if_let_guard, io_error_more, let_chains)]
#![cfg_attr(feature = "deny-warnings", deny(warnings))]
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

extern crate rustc_ast;
extern crate rustc_data_structures;
#[allow(unused_extern_crates)]
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_session;
extern crate rustc_span;

#[macro_use]
mod de;
mod conf;
mod metadata;
pub mod msrvs;
pub mod types;

pub use conf::Conf;
pub use metadata::ConfMetadata;

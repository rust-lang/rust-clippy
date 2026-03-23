#![feature(rustc_private)]

pub mod deprecated_lints;
pub mod lints;

pub use deprecated_lints::{DEPRECATED, DEPRECATED_VERSION, RENAMED, RENAMED_VERSION};
pub use lints::LINTS;

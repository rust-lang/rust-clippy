#![warn(clippy::use_crate_prefix_for_self_imports)]

#[cfg(feature = "bar")]
mod foo;
#[cfg(feature = "bar")]
pub use foo::Foo;

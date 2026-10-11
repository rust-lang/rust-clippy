#![warn(clippy::use_crate_prefix_for_self_imports)]

mod foo;
#[doc(hidden)]
pub use foo::Foo;

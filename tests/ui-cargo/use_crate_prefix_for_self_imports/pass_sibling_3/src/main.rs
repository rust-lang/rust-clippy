#![warn(clippy::use_crate_prefix_for_self_imports)]

mod foo;
pub use foo::{Bar, Foo};

fn main() {
    let _foo = Foo;
    let _bar = Bar;
}

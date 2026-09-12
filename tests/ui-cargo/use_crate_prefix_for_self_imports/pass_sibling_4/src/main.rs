#![warn(clippy::use_crate_prefix_for_self_imports)]

pub use foo::{Bar, Foo};
mod foo;

fn main() {
    let _foo = Foo;
    let _bar = Bar;
}

#![warn(clippy::use_crate_prefix_for_self_imports)]

mod foo;
//fadsfsadfa
use foo::Foo;

fn main() {
    let _foo = Foo;
}

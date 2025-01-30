#![warn(clippy::use_crate_prefix_for_self_imports)]

use foo::Foo;
mod foo;

fn main() {
    let _foo = Foo;
}

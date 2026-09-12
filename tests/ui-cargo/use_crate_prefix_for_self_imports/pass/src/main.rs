#![warn(clippy::use_crate_prefix_for_self_imports)]

use crate::foo::Foo;

mod foo;

fn main() {
    let _foo = Foo;
}

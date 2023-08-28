#![warn(clippy::enum_glob_use)]
#![allow(unused)]
#![warn(unused_imports)]

use std::cmp::Ordering::*;
//~^ ERROR: usage of wildcard import for enum variants
//~| NOTE: `-D clippy::enum-glob-use` implied by `-D warnings`

enum Enum {
    Foo,
}

use self::Enum::*;
//~^ ERROR: usage of wildcard import for enum variants

mod in_fn_test {
    fn blarg() {
        use crate::Enum::*;
        //~^ ERROR: usage of wildcard import for enum variants

        let _ = Foo;
    }
}

mod blurg {
    pub use std::cmp::Ordering::*; // ok, re-export
}

fn main() {
    let _ = Foo;
    let _ = Less;
}

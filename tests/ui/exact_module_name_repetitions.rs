//@compile-flags: --test

#![warn(clippy::exact_module_name_repetitions)]
#![allow(dead_code)]

pub mod foo {
    pub fn foo() {}
    //~^ exact_module_name_repetitions

    pub struct Foo;
    //~^ exact_module_name_repetitions
}

pub mod bar {
    // Shouldn't warn when item is declared in a private module...
    mod baz {
        pub struct Bar;
    }
    // ... but should still warn when the item is reexported to create a *public* path with repetition.
    pub use baz::Bar;
    //~^ exact_module_name_repetitions
}

pub mod baz {
    // FIXME: This should also warn because it creates the public path `baz::Baz`.
    mod inner {
        pub struct Baz;
    }
    pub use inner::*;
}

fn main() {}

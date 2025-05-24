//@ check-pass
#![expect(clippy::disallowed_names)]
#![allow(clippy::relative_path_in_macro_definition)]

// Test for https://github.com/rust-lang/rust-clippy/issues/3462

enum Foo {
    Bar,
    Baz,
}

fn bar(foo: Foo) {
    macro_rules! baz {
        () => {
            if let Foo::Bar = foo {}
        };
    }

    baz!();
    baz!();
}

fn main() {}

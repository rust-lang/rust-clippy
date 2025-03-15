//@aux-build:external_macro.rs
#![warn(clippy::struct_fields_rest_default)]

extern crate external_macro;

use external_macro::external_struct_rest_default;

#[derive(Default)]
struct Foo {
    a: i32,
    b: i32,
    c: i32,
}

fn main() {
    #[rustfmt::skip]
    let _ = Foo {
        a: 10,
        ..Default::default()
        //~^ struct_fields_rest_default
    };

    // should not lint in external macro
    external_struct_rest_default!();

    // should not lint
    let _ = Foo {
        a: Default::default(),
        b: Default::default(),
        c: Default::default(),
    };
}

//@aux-build:proc_macros.rs
#![warn(clippy::struct_fields_rest_default)]
extern crate proc_macros;

#[derive(Default)]
struct Foo {
    a: i32,
    b: i32,
    c: i32,
}

impl Foo {
    fn get_foo() -> Self {
        Foo { a: 0, b: 0, c: 0 }
    }
}

fn main() {
    #[rustfmt::skip]
    let _ = Foo {
        a: 10,
        ..Default::default()
        //~^ struct_fields_rest_default
    };

    #[rustfmt::skip]
    let _ = Foo {
        a: 10,
        ..Foo::default()
        //~^ struct_fields_rest_default
    };

    // should not lint
    #[rustfmt::skip]
    let _ = Foo {
        a: 10,
        ..Foo::get_foo()
    };

    // should not lint in external macro
    proc_macros::external! {
        #[derive(Default)]
        struct ExternalDefault {
            a: i32,
            b: i32,
        }

        let _ = ExternalDefault {
            a: 10,
            ..Default::default()
        };
    }

    // should not lint
    let _ = Foo {
        a: Default::default(),
        b: Default::default(),
        c: Default::default(),
    };
}

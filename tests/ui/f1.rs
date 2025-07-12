#![allow(clippy::disallowed_names)]
#![warn(clippy::map_identity)]

#[derive(Clone, Copy)]
struct Foo {
    foo: u8,
    bar: u8,
}

struct Bar {
    foo: u8,
    bar: u8,
}

fn main() {
    let x = [Foo { foo: 0, bar: 0 }];

    let _ = x.into_iter().map(|Foo { foo, bar }| Foo { foo, bar });
    //~^ map_identity

    // don't lint: same fields but different structs
    let _ = x.into_iter().map(|Foo { foo, bar }| Bar { foo, bar });

    // still lint with redundant field names
    #[allow(clippy::redundant_field_names)]
    let _ = x.into_iter().map(|Foo { foo, bar }| Foo { foo: foo, bar: bar });
    //~^ map_identity

    // still lint with field order change
    let _ = x.into_iter().map(|Foo { foo, bar }| Foo { bar, foo });
    //~^ map_identity

    // don't lint: switched field assignment
    let _ = x.into_iter().map(|Foo { foo, bar }| Foo { foo: bar, bar: foo });
}

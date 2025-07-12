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
}

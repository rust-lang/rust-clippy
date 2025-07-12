#![allow(clippy::disallowed_names)]
#![warn(clippy::map_identity)]

struct Foo {
    foo: u8,
    bar: u8,
}

fn main() {
    let x = [Foo { foo: 0, bar: 0 }];

    let _ = x.into_iter().map(|Foo { foo, bar }| Foo { foo, bar });
    //~^ map_identity
}

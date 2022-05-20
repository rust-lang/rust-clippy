#[allow(clippy::unnecessary_operation)]
#[allow(clippy::implicit_clone)]

fn main() {
    let x = &Baz;
    let y = &Baz;
    y.to_owned() == *x;

    let x = &&Baz;
    let y = &Baz;
    y.to_owned() == **x;
}

struct Foo;

impl PartialEq for Foo {
    fn eq(&self, other: &Self) -> bool {
        self.to_owned() == *other
    }
}

impl ToOwned for Foo {
    type Owned = Bar;
    fn to_owned(&self) -> Bar {
        Bar
    }
}

#[derive(PartialEq, Eq)]
struct Baz;

impl ToOwned for Baz {
    type Owned = Baz;
    fn to_owned(&self) -> Baz {
        Baz
    }
}

#[derive(PartialEq, Eq)]
struct Bar;

impl PartialEq<Foo> for Bar {
    fn eq(&self, _: &Foo) -> bool {
        true
    }
}

impl std::borrow::Borrow<Foo> for Bar {
    fn borrow(&self) -> &Foo {
        static FOO: Foo = Foo;
        &FOO
    }
}

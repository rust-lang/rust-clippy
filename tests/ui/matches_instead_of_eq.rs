#![warn(clippy::matches_instead_of_eq)]

enum Foo {
    Bar,
    Baz,
}

#[derive(PartialEq)]
enum EvenOdd {
    Even,
    Odd,
    Unknown,
}

#[derive(PartialEq)]
enum Foo2 {
    Bar(u8, u8),
    Baz { x: u8, y: u8 },
}

fn main() {
    let x = Foo::Bar;
    let val = matches!(x, Foo::Bar); // No error as Foo does not implement PartialEq

    let x = EvenOdd::Even;
    let val = matches!(x, EvenOdd::Even);
    //~^ matches_instead_of_eq

    let x = EvenOdd::Odd;
    let val = matches!(x, EvenOdd::Even | EvenOdd::Odd); // No error

    let x = Foo2::Bar(1, 2);
    let val = matches!(x, Foo2::Bar(_, _)); // No error

    let x = Foo2::Baz { x: 1, y: 2 };
    let val = matches!(x, Foo2::Baz { .. }); // No Error

    let val = matches!(x, Foo2::Bar(..) | Foo2::Baz { .. }); // No error
}

#![warn(clippy::too_many_fields)]

struct S {
    //~^ too_many_fields
    a: u8,
}

struct Tuple(
    //~^ too_many_fields
    u8,
);

struct Unit;

fn main() {}

#![warn(clippy::drop_for_static)]
#![allow(unused)]

struct FooWithDrop;
struct FooWithoutDrop;

impl Drop for FooWithDrop {
    fn drop(&mut self) {}
}

static A1: FooWithDrop = FooWithDrop;
//~^ drop_for_static
static A2: FooWithoutDrop = FooWithoutDrop;

static A3: [FooWithDrop; 1] = [FooWithDrop];
//~^ drop_for_static
static A4: [FooWithoutDrop; 1] = [FooWithoutDrop];

fn main() {}

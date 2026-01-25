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

static A3: &FooWithDrop = &FooWithDrop;
//~^ drop_for_static
static A4: &FooWithoutDrop = &FooWithoutDrop;

static A5: (FooWithoutDrop, FooWithDrop) = (FooWithoutDrop, FooWithDrop);
//~^ drop_for_static
static A6: (FooWithoutDrop, FooWithoutDrop) = (FooWithoutDrop, FooWithoutDrop);

static A7: [FooWithDrop; 1] = [FooWithDrop];
//~^ drop_for_static
static A8: [FooWithoutDrop; 1] = [FooWithoutDrop];

static A9: &[FooWithDrop] = &[FooWithDrop];
//~^ drop_for_static
static A10: &[FooWithoutDrop] = &[FooWithoutDrop];

struct Nested<T>(T);
static B9: Nested<FooWithDrop> = Nested(FooWithDrop);
static B10: Nested<FooWithoutDrop> = Nested(FooWithoutDrop);

fn main() {}

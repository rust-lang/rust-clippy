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

// ----------------------
// nested types scenarios
// ----------------------

struct Nested<T1, T2 = ()>(T1, T2);
static B1: Nested<FooWithDrop> = Nested(FooWithDrop, ());
//~^ drop_for_static
static B2: Nested<FooWithoutDrop> = Nested(FooWithoutDrop, ());

static B3: Nested<FooWithoutDrop, Nested<FooWithDrop>> = Nested(FooWithoutDrop, Nested(FooWithDrop, ()));
//~^ drop_for_static
static B4: Nested<FooWithoutDrop, Nested<FooWithoutDrop>> = Nested(FooWithoutDrop, Nested(FooWithoutDrop, ()));

static B5: Nested<&FooWithDrop> = Nested(&FooWithDrop, ());
//~^ drop_for_static
static B6: Nested<&FooWithoutDrop> = Nested(&FooWithoutDrop, ());

static B7: Nested<(FooWithoutDrop, FooWithDrop)> = Nested((FooWithoutDrop, FooWithDrop), ());
//~^ drop_for_static
static B8: Nested<(FooWithoutDrop, FooWithoutDrop)> = Nested((FooWithoutDrop, FooWithoutDrop), ());

static B9: Nested<[FooWithDrop; 1]> = Nested([FooWithDrop], ());
//~^ drop_for_static
static B10: Nested<[FooWithoutDrop; 1]> = Nested([FooWithoutDrop], ());

static B11: Nested<&[FooWithDrop]> = Nested(&[FooWithDrop], ());
//~^ drop_for_static
static B12: Nested<&[FooWithoutDrop]> = Nested(&[FooWithoutDrop], ());

fn main() {}

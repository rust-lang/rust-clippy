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

static A3: (FooWithoutDrop, FooWithDrop) = (FooWithoutDrop, FooWithDrop);
//~^ drop_for_static
static A4: (FooWithoutDrop, FooWithoutDrop) = (FooWithoutDrop, FooWithoutDrop);

static A5: [FooWithDrop; 1] = [FooWithDrop];
//~^ drop_for_static
static A6: [FooWithoutDrop; 1] = [FooWithoutDrop];

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

static B5: Nested<(FooWithoutDrop, FooWithDrop)> = Nested((FooWithoutDrop, FooWithDrop), ());
//~^ drop_for_static
static B6: Nested<(FooWithoutDrop, FooWithoutDrop)> = Nested((FooWithoutDrop, FooWithoutDrop), ());

static B7: Nested<[FooWithDrop; 1]> = Nested([FooWithDrop], ());
//~^ drop_for_static
static B8: Nested<[FooWithoutDrop; 1]> = Nested([FooWithoutDrop], ());

// ----------------------
// type alias
// ----------------------

type BarWithDrop = FooWithDrop;
type BarWithoutDrop = FooWithoutDrop;

static C1: BarWithDrop = FooWithDrop;
//~^ drop_for_static
static C2: BarWithoutDrop = FooWithoutDrop;

static C3: (BarWithoutDrop, BarWithDrop) = (FooWithoutDrop, FooWithDrop);
//~^ drop_for_static
static C4: (BarWithoutDrop, BarWithoutDrop) = (FooWithoutDrop, FooWithoutDrop);

static C5: [BarWithDrop; 1] = [FooWithDrop];
//~^ drop_for_static
static C6: [BarWithoutDrop; 1] = [FooWithoutDrop];

// ----------------------
// nested type alias
// ----------------------

static D1: Nested<BarWithDrop> = Nested(FooWithDrop, ());
//~^ drop_for_static
static D2: Nested<BarWithoutDrop> = Nested(FooWithoutDrop, ());

static D3: Nested<BarWithoutDrop, Nested<BarWithDrop>> = Nested(FooWithoutDrop, Nested(FooWithDrop, ()));
//~^ drop_for_static
static D4: Nested<BarWithoutDrop, Nested<BarWithoutDrop>> = Nested(FooWithoutDrop, Nested(FooWithoutDrop, ()));

static D5: Nested<(BarWithoutDrop, BarWithDrop)> = Nested((FooWithoutDrop, FooWithDrop), ());
//~^ drop_for_static
static D6: Nested<(BarWithoutDrop, BarWithoutDrop)> = Nested((FooWithoutDrop, FooWithoutDrop), ());

static D7: Nested<[BarWithDrop; 1]> = Nested([FooWithDrop], ());
//~^ drop_for_static
static D8: Nested<[BarWithoutDrop; 1]> = Nested([FooWithoutDrop], ());

fn main() {}

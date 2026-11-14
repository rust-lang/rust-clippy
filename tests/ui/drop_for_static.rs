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
// generic type scenarios
// ----------------------

struct Generic<T1, T2 = ()>(T1, T2);
static B1: Generic<FooWithDrop> = Generic(FooWithDrop, ());
//~^ drop_for_static
static B2: Generic<FooWithoutDrop> = Generic(FooWithoutDrop, ());

static B3: Generic<FooWithoutDrop, Generic<FooWithDrop>> = Generic(FooWithoutDrop, Generic(FooWithDrop, ()));
//~^ drop_for_static
static B4: Generic<FooWithoutDrop, Generic<FooWithoutDrop>> = Generic(FooWithoutDrop, Generic(FooWithoutDrop, ()));

static B5: Generic<(FooWithoutDrop, FooWithDrop)> = Generic((FooWithoutDrop, FooWithDrop), ());
//~^ drop_for_static
static B6: Generic<(FooWithoutDrop, FooWithoutDrop)> = Generic((FooWithoutDrop, FooWithoutDrop), ());

static B7: Generic<[FooWithDrop; 1]> = Generic([FooWithDrop], ());
//~^ drop_for_static
static B8: Generic<[FooWithoutDrop; 1]> = Generic([FooWithoutDrop], ());

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
// Generic type with alias
// ----------------------

static D1: Generic<BarWithDrop> = Generic(FooWithDrop, ());
//~^ drop_for_static
static D2: Generic<BarWithoutDrop> = Generic(FooWithoutDrop, ());

static D3: Generic<BarWithoutDrop, Generic<BarWithDrop>> = Generic(FooWithoutDrop, Generic(FooWithDrop, ()));
//~^ drop_for_static
static D4: Generic<BarWithoutDrop, Generic<BarWithoutDrop>> = Generic(FooWithoutDrop, Generic(FooWithoutDrop, ()));

static D5: Generic<(BarWithoutDrop, BarWithDrop)> = Generic((FooWithoutDrop, FooWithDrop), ());
//~^ drop_for_static
static D6: Generic<(BarWithoutDrop, BarWithoutDrop)> = Generic((FooWithoutDrop, FooWithoutDrop), ());

static D7: Generic<[BarWithDrop; 1]> = Generic([FooWithDrop], ());
//~^ drop_for_static
static D8: Generic<[BarWithoutDrop; 1]> = Generic([FooWithoutDrop], ());

// ----------------------
// associated type
// ----------------------

trait FooTrait {
    type FooAssoc;
}

struct FooImplWithDrop;
impl FooTrait for FooImplWithDrop {
    type FooAssoc = FooWithDrop;
}
struct FooImplWithoutDrop;
impl FooTrait for FooImplWithoutDrop {
    type FooAssoc = FooWithoutDrop;
}

static F1: <FooImplWithDrop as FooTrait>::FooAssoc = FooWithDrop;
//~^ drop_for_static
static F2: <FooImplWithoutDrop as FooTrait>::FooAssoc = FooWithoutDrop;

static F3: (
    //~^ drop_for_static
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
    <FooImplWithDrop as FooTrait>::FooAssoc,
) = (FooWithoutDrop, FooWithDrop);
static F4: (
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
) = (FooWithoutDrop, FooWithoutDrop);

static F5: [<FooImplWithDrop as FooTrait>::FooAssoc; 1] = [FooWithDrop];
//~^ drop_for_static
static F6: [<FooImplWithoutDrop as FooTrait>::FooAssoc; 1] = [FooWithoutDrop];

// ----------------------
// generic with associated type scenarios
// ----------------------

static E1: Generic<<FooImplWithDrop as FooTrait>::FooAssoc> = Generic(FooWithDrop, ());
//~^ drop_for_static
static E2: Generic<<FooImplWithoutDrop as FooTrait>::FooAssoc> = Generic(FooWithoutDrop, ());

static E3: Generic<<FooImplWithoutDrop as FooTrait>::FooAssoc, Generic<<FooImplWithDrop as FooTrait>::FooAssoc>> =
    //~^ drop_for_static
    Generic(FooWithoutDrop, Generic(FooWithDrop, ()));
static E4: Generic<<FooImplWithoutDrop as FooTrait>::FooAssoc, Generic<FooWithoutDrop>> =
    Generic(FooWithoutDrop, Generic(FooWithoutDrop, ()));

static E5: Generic<(
    //~^ drop_for_static
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
    <FooImplWithDrop as FooTrait>::FooAssoc,
)> = Generic((FooWithoutDrop, FooWithDrop), ());
static E6: Generic<(
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
    <FooImplWithoutDrop as FooTrait>::FooAssoc,
)> = Generic((FooWithoutDrop, FooWithoutDrop), ());

static E7: Generic<[<FooImplWithDrop as FooTrait>::FooAssoc; 1]> = Generic([FooWithDrop], ());
//~^ drop_for_static
static E8: Generic<[<FooImplWithoutDrop as FooTrait>::FooAssoc; 1]> = Generic([FooWithoutDrop], ());

fn main() {}

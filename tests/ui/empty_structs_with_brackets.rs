#![warn(clippy::empty_structs_with_brackets)]
#![allow(dead_code)]

pub struct MyEmptyStruct {} // should trigger lint
//~^ ERROR: found empty brackets on struct declaration
//~| NOTE: `-D clippy::empty-structs-with-brackets` implied by `-D warnings`
struct MyEmptyTupleStruct(); // should trigger lint
//~^ ERROR: found empty brackets on struct declaration

// should not trigger lint
struct MyCfgStruct {
    #[cfg(feature = "thisisneverenabled")]
    field: u8,
}

// should not trigger lint
struct MyCfgTupleStruct(#[cfg(feature = "thisisneverenabled")] u8);

// should not trigger lint
struct MyStruct {
    field: u8,
}
struct MyTupleStruct(usize, String); // should not trigger lint
struct MySingleTupleStruct(usize); // should not trigger lint
struct MyUnitLikeStruct; // should not trigger lint

fn main() {}

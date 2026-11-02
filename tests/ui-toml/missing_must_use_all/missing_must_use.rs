//@aux-build:../../ui/auxiliary/missing_must_use_macro.rs
#![warn(clippy::missing_must_use)]

pub struct PublicFields(u8);
//~^ missing_must_use

struct PrivateFields(u8);
//~^ missing_must_use

pub(crate) struct CrateFields(u8);
//~^ missing_must_use

pub struct PublicUnit;
//~^ missing_must_use

pub struct PublicEmptyTuple();
//~^ missing_must_use

pub struct PublicEmptyStruct {}
//~^ missing_must_use

struct PrivateUnit;
//~^ missing_must_use

struct PrivateEmptyTuple();
//~^ missing_must_use

struct PrivateEmptyStruct {}
//~^ missing_must_use

pub enum PublicSingleUnit {
    //~^ missing_must_use
    Unit,
}

pub enum PublicSingleEmptyTuple {
    //~^ missing_must_use
    Tuple(),
}

pub enum PublicSingleEmptyStruct {
    //~^ missing_must_use
    Struct {},
}

enum PrivateSingleUnit {
    //~^ missing_must_use
    Unit,
}

pub enum PublicSingleFields {
    //~^ missing_must_use
    Tuple(u8),
}

enum PrivateSingleFields {
    //~^ missing_must_use
    Tuple(u8),
}

pub enum PublicMultipleUnits {
    //~^ missing_must_use
    First,
    Second,
}

enum PrivateMultipleUnits {
    //~^ missing_must_use
    First,
    Second,
}

pub union PublicUnion {
    //~^ missing_must_use
    value: u8,
}

union PrivateUnion {
    //~^ missing_must_use
    value: u8,
}

mod hidden {
    pub struct Fields(u8);
    //~^ missing_must_use
}

// Uninhabited enums are never checked, even with both options enabled.
pub enum PublicUninhabited {}
enum PrivateUninhabited {}

#[must_use]
pub struct AnnotatedFields(u8);
#[must_use]
struct AnnotatedPrivateFields(u8);
#[must_use]
pub struct AnnotatedUnit;
#[must_use]
struct AnnotatedPrivateUnit;
#[must_use]
pub enum AnnotatedSingleUnit {
    Unit,
}
#[must_use]
enum AnnotatedPrivateSingleUnit {
    Unit,
}

pub trait IgnoredTrait {}
pub type IgnoredAlias = PublicFields;
pub fn ignored_function() -> u8 {
    0
}

extern crate missing_must_use_macro;
missing_must_use_macro::define_types!();

fn main() {}

// aux-build:not_exhaustive_enough_helper.rs

#![warn(clippy::not_exhaustive_enough)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::never_loop)]
#![allow(clippy::single_match)]

extern crate not_exhaustive_enough_helper;
use not_exhaustive_enough_helper::{AnotherCrateEnum, AnotherCrateStruct, TPrivateField};

#[non_exhaustive]
pub enum DefaultEnum {
    First,
    Second,
    Third,
}

#[non_exhaustive]
pub enum DataEnum {
    First(String),
    Second(u32, u32),
    Third(String),
}

pub enum StructVariantEnum1 {
    #[non_exhaustive]
    V { a: i32, b: i32 },
}

pub enum StructVariantEnum2 {
    #[non_exhaustive]
    V {
        a: i32,
        b: i32,
    },
    A {
        c: u32,
    },
}

#[derive(Default)]
#[non_exhaustive]
pub struct DefaultStruct {
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

#[derive(Default)]
#[non_exhaustive]
pub struct DefaultTuple(pub i32, pub i32, pub i32);

fn main() {
    //////// Enum

    let default_enum = DefaultEnum::First;

    let struct_variant_enum_1 = StructVariantEnum1::V { a: 1, b: 2 };

    let struct_variant_enum_2 = StructVariantEnum2::V { a: 1, b: 2 };

    match default_enum {
        DefaultEnum::First => {},
        DefaultEnum::Second => {},
        _ => {},
    }

    match struct_variant_enum_1 {
        StructVariantEnum1::V { a: _, .. } => {},
    }

    match struct_variant_enum_2 {
        StructVariantEnum2::V { a: _, .. } => {},
        _ => {},
    }

    //

    let example = "Example".to_string();
    let data_enum = DataEnum::First(example);

    match data_enum {
        DataEnum::First(..) => {},
        DataEnum::Second(..) => {},
        _ => {},
    }

    //////// Struct

    let DefaultStruct { a: _, b: _, .. } = DefaultStruct::default();

    match DefaultStruct::default() {
        DefaultStruct { a: 42, b: 21, .. } => {},
        DefaultStruct { a: _, b: _, .. } => {},
    }

    if let DefaultStruct { a: 42, b: _, .. } = DefaultStruct::default() {}

    let v = vec![DefaultStruct::default()];

    for DefaultStruct { a: _, b: _, .. } in v {}

    while let DefaultStruct { a: 42, b: _, .. } = DefaultStruct::default() {
        break;
    }

    pub fn take_s(DefaultStruct { a, b, .. }: DefaultStruct) -> (i32, i32) {
        (a, b)
    }

    //////// Tuple Struct

    let DefaultTuple { 0: _, 1: _, .. } = DefaultTuple::default();

    match DefaultTuple::default() {
        DefaultTuple { 0: 42, 1: 21, .. } => {},
        DefaultTuple { 0: _, 1: _, .. } => {},
    }

    if let DefaultTuple { 0: 42, 1: _, .. } = DefaultTuple::default() {}

    let default_tuple = vec![DefaultTuple::default()];
    for DefaultTuple { 0: _, 1: _, .. } in default_tuple {}

    while let DefaultTuple { 0: 42, 1: _, .. } = DefaultTuple::default() {
        break;
    }

    pub fn take_t(DefaultTuple { 0: _, 1: _, .. }: DefaultTuple) -> (i32, i32) {
        (0, 1)
    }

    //////// Tuple Struct - private field

    let TPrivateField { 0: _, 1: _, .. } = TPrivateField::default();

    match TPrivateField::default() {
        TPrivateField { 0: 42, 1: 21, .. } => {},
        TPrivateField { 0: _, 1: _, .. } => {},
    }

    match TPrivateField::default() {
        TPrivateField { 1: 21, .. } => {},
        _ => {},
    }

    if let TPrivateField { 0: 42, 1: _, .. } = TPrivateField::default() {}

    let m = vec![TPrivateField::default()];
    for TPrivateField { 0: _, 1: _, .. } in m {}

    while let TPrivateField { 0: 42, 1: _, .. } = TPrivateField::default() {
        break;
    }

    pub fn take_w(TPrivateField { 0: _, 1: _, .. }: TPrivateField) -> (i32, i32) {
        (0, 1)
    }

    // Enum - Another Crate

    let another_crate_enum = AnotherCrateEnum::AFirst;

    match another_crate_enum {
        AnotherCrateEnum::AFirst => {},
        _ => {},
    }

    // Struct - Another Crate

    let AnotherCrateStruct { a1: _, b1: _, .. } = AnotherCrateStruct::default();

    match AnotherCrateStruct::default() {
        AnotherCrateStruct { a1: 42, b1: 21, .. } => {},
        AnotherCrateStruct { a1: _, b1: _, .. } => {},
    }

    if let AnotherCrateStruct { a1: 42, b1: _, .. } = AnotherCrateStruct::default() {}

    let a_v = vec![AnotherCrateStruct::default()];

    for AnotherCrateStruct { a1: _, b1: _, .. } in a_v {}

    while let AnotherCrateStruct { a1: 42, b1: _, .. } = AnotherCrateStruct::default() {
        break;
    }

    pub fn take_a_s(AnotherCrateStruct { a1, b1, .. }: AnotherCrateStruct) -> (i32, i32) {
        (a1, b1)
    }
}

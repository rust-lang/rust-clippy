// aux-build:not_exhaustive_enough_helper.rs

#![warn(clippy::not_exhaustive_enough)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::never_loop)]
#![allow(clippy::single_match)]

extern crate not_exhaustive_enough_helper;
use not_exhaustive_enough_helper::{AnotherCrateEnum, AnotherCrateStruct, TPrivateField};

#[non_exhaustive]
pub enum E {
    First,
    Second,
    Third,
}

#[non_exhaustive]
pub enum K {
    First(String),
    Second(u32, u32),
    Third(String)
}

enum EF {
    #[non_exhaustive]
    V{a: i32, b: i32}
}

enum F {
    #[non_exhaustive]
    V{a: i32, b: i32},
    A{c: u32}
}

#[derive(Default)]
#[non_exhaustive]
pub struct S {
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

#[derive(Default)]
#[non_exhaustive]
pub struct T(pub i32, pub i32, pub i32);


fn main() {
    //////// Enum

    let e = E::First;

    let ef = EF::V{a: 1, b:2};

    let f = F::V{a:1, b:2};

    match e {
        E::First => {},
        E::Second => {},
        _ => {},
    }

    match ef {
        EF::V{a:_, ..} => {}
    }

    if let F::V{a:_, ..} = f {}

    //
    let example = "Example".to_string();
    let k = K::First(example);

    match k {
        K::First(..) => {},
        K::Second(..) => {},
        _ => {},
    }

    //////// Struct

    let S { a: _, b: _, .. } = S::default();

    match S::default() {
        S { a: 42, b: 21, .. } => {},
        S { a: _, b: _, .. } => {},
    }

    if let S { a: 42, b: _, .. } = S::default() {}

    let v = vec![S::default()];

    for S { a: _, b: _, .. } in v {}

    while let S { a: 42, b: _, .. } = S::default() {
        break;
    }

    pub fn take_s(S { a, b, .. }: S) -> (i32, i32) {
        (a, b)
    }

    //////// Tuple Struct

    let T { 0: _, 1: _, .. } = T::default();

    match T::default() {
        T { 0: 42, 1: 21, .. } => {},
        T { 0: _, 1: _, .. } => {},
    }

    if let T { 0: 42, 1: _, .. } = T::default() {}

    let v = vec![T::default()];
    for T { 0: _, 1: _, .. } in v {}

    while let T { 0: 42, 1: _, .. } = T::default() {
        break;
    }

    pub fn take_t(T { 0: _, 1: _, .. }: T) -> (i32, i32) {
        (0, 1)
    }

    //////// Tuple Struct - private field

    let TPrivateField { 0: _, 1: _, .. } = TPrivateField::default();

    match TPrivateField::default() {
        TPrivateField { 0: 42, 1: 21, .. } => {},
        TPrivateField { 0: _, 1: _, .. } => {},
    }

    match TPrivateField::default() {
        TPrivateField {1: 21, .. } => {},
        _ => {}
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

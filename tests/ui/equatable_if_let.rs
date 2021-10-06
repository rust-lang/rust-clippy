// run-rustfix

#![allow(unused_variables, dead_code, clippy::redundant_pattern_matching, clippy::op_ref)]
#![warn(clippy::equatable_if_let, clippy::equatable_matches)]

use std::cmp::Ordering;

#[derive(PartialEq)]
enum Enum {
    TupleVariant(i32, u64),
    RecordVariant { a: i64, b: u32 },
    UnitVariant,
    Recursive(Struct),
}

#[derive(PartialEq)]
struct Struct {
    a: i32,
    b: bool,
}

#[derive(Clone, Copy)]
enum NotPartialEq {
    A,
    B,
}

#[derive(Clone, Copy)]
enum NotStructuralEq {
    A,
    B,
}

impl PartialEq for NotStructuralEq {
    fn eq(&self, _: &NotStructuralEq) -> bool {
        false
    }
}

#[derive(PartialEq)]
enum Generic<A, B> {
    VA(A),
    VB(B),
    VC,
}

#[derive(PartialEq)]
struct Generic2<A, B> {
    a: A,
    b: B,
}

fn macro_pattern() {
    macro_rules! m1 {
        (x) => {
            "abc"
        };
    }
    if let m1!(x) = "abc" {
        println!("OK");
    }
}

fn main() {
    let a = 2;
    let b = 3;
    let c = Some(2);
    let d = Struct { a: 2, b: false };
    let e = Enum::UnitVariant;
    let f = NotPartialEq::A;
    let g = NotStructuralEq::A;
    let h: Generic<Enum, NotPartialEq> = Generic::VC;
    let i: Generic<Enum, NotStructuralEq> = Generic::VC;
    let j = vec![1, 2, 3, 4];
    let k = Some(&false);
    let l = Generic2 {
        a: Generic2 { a: "xxxx", b: 3 },
        b: Generic2 {
            a: &Enum::UnitVariant,
            b: false,
        },
    };
    let m = Generic2 { a: 3, b: 5 };
    let n = Some("xxxx");
    let mut o = j.iter();

    // true

    if let 2 = a {}
    if let "hello" = "world" {}
    if let Ordering::Greater = a.cmp(&b) {}
    if let Enum::UnitVariant = e {}
    if let None = Some(g) {}
    if let Generic::VC = i {}
    if let None = k {}

    let _ = matches!(b, 2);

    // false

    if let Some(2) = c {}
    if let Struct { a: 2, b: false } = d {}
    if let Enum::TupleVariant(32, 64) = e {}
    if let Enum::RecordVariant { a: 64, b: 32 } = e {}
    if let Some("yyy") = n {}

    let _ = matches!(c, Some(2));

    while let Some(4 | 7) = o.next() {}
}

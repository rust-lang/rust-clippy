#![allow(dead_code)]
#![warn(clippy::arbitrary_source_item_ordering)]

/// This module gets linted before clippy gives up.
mod i_am_just_right {
    const BEFORE: i8 = 0;

    const AFTER: i8 = 0;
}

// Use statements should not be linted internally - this is normally auto-sorted using rustfmt.
use std::rc::Rc;
use std::sync::{Arc, Barrier, RwLock};

const SNAKE_CASE: &str = "zzzzzzzz";

use std::rc::Weak;

enum EnumOrdered {
    A,
    B,
    C,
}

enum EnumUnordered {
    A,
    C,
    B,
}

#[allow(clippy::arbitrary_source_item_ordering)]
enum EnumUnorderedAllowed {
    A,
    C,
    B,
}

struct StructOrdered {
    a: bool,
    b: bool,
    c: bool,
}

struct StructUnordered {
    a: bool,
    c: bool,
    b: bool,
    d: bool,
}

struct StructUnorderedGeneric<T> {
    _1: std::marker::PhantomData<T>,
    a: bool,
    c: bool,
    b: bool,
    d: bool,
}

trait TraitOrdered {
    const A: bool;
    const B: bool;
    const C: bool;

    type SomeType;

    fn a();
    fn b();
    fn c();
}

trait TraitUnordered {
    const A: bool;
    const C: bool;
    const B: bool;

    type SomeType;

    fn a();
    fn c();
    fn b();
}

trait TraitUnorderedItemKinds {
    type SomeType;

    const A: bool;
    const B: bool;
    const C: bool;

    fn a();
    fn b();
    fn c();
}

impl TraitUnordered for StructUnordered {
    const A: bool = false;
    const C: bool = false;
    const B: bool = false;

    type SomeType = ();

    fn a() {}
    fn c() {}
    fn b() {}
}

impl TraitUnorderedItemKinds for StructUnordered {
    type SomeType = ();

    const A: bool = false;
    const B: bool = false;
    const C: bool = false;

    fn a() {}
    fn b() {}
    fn c() {}
}

const ZIS_SHOULD_BE_REALLY_EARLY: () = ();

fn main() {
    // test code goes here
}

/// Note that the linting pass is stopped before recursing into this module.
mod this_is_in_the_wrong_position {
    const C: i8 = 0;
    const A: i8 = 1;
}

struct ZisShouldBeBeforeZeMainFn;

const ZIS_SHOULD_BE_EVEN_EARLIER: () = ();

#[cfg(test)]
mod test {
    const B: i8 = 1;

    const A: i8 = 0;
}

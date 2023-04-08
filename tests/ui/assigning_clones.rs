// run-rustfix
#![allow(unused)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::ptr_arg)] // https://github.com/rust-lang/rust-clippy/issues/10612
#![warn(clippy::assigning_clones)]

use std::borrow::ToOwned;
use std::ops::Add;

#[derive(Clone)]
pub struct Thing;

pub fn val_to_mut_ref(mut_thing: &mut Thing, value_thing: Thing) {
    *mut_thing = value_thing.clone();
}

pub fn ref_to_mut_ref(mut_thing: &mut Thing, ref_thing: &Thing) {
    *mut_thing = ref_thing.clone();
}

pub fn to_op(mut_thing: &mut Thing, ref_thing: &Thing) {
    // These parens should be kept as necessary for a receiver
    *(mut_thing + &mut Thing) = ref_thing.clone();
}

pub fn from_op(mut_thing: &mut Thing, ref_thing: &Thing) {
    // These parens should be removed since they are not needed in a function argument
    *mut_thing = (ref_thing + ref_thing).clone();
}

pub fn assign_to_var(b: Thing) -> Thing {
    let mut a = Thing;
    for _ in 1..10 {
        // TODO: We should rewrite this, but aren't checking whether `a` is initialized
        // to be certain it's okay.
        a = b.clone();
    }
    a
}

pub fn assign_to_uninit_var(b: Thing) -> Thing {
    // This case CANNOT be rewritten as clone_from() and should not warn.
    let mut a;
    a = b.clone();
    a
}

/// Test `ToOwned` as opposed to `Clone`.
pub fn toowned_to_mut_ref(mut_string: &mut String, ref_str: &str) {
    *mut_string = ref_str.to_owned();
}

/// Don't remove the dereference operator here (as we would for clone_from); we need it.
pub fn toowned_to_derefed(mut_box_string: &mut Box<String>, ref_str: &str) {
    // TODO: This is not actually linted, but it should be.
    **mut_box_string = ref_str.to_owned();
}

struct FakeClone;
impl FakeClone {
    /// This looks just like Clone::clone but has no clone_from()
    fn clone(&self) -> Self {
        FakeClone
    }
}

pub fn test_fake_clone() {
    let mut a = FakeClone;
    let b = FakeClone;
    // This should not be changed because it is not Clone::clone
    a = b.clone();
}

fn main() {}

/// Trait implementation to allow producing a `Thing` with a low-precedence expression.
impl Add for Thing {
    type Output = Self;
    fn add(self, _: Thing) -> Self {
        self
    }
}
/// Trait implementation to allow producing a `&Thing` with a low-precedence expression.
impl<'a> Add for &'a Thing {
    type Output = Self;
    fn add(self, _: &'a Thing) -> Self {
        self
    }
}
/// Trait implementation to allow producing a `&mut Thing` with a low-precedence expression.
impl<'a> Add for &'a mut Thing {
    type Output = Self;
    fn add(self, _: &'a mut Thing) -> Self {
        self
    }
}

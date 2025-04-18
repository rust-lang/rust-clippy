//@aux-build:proc_macro_derive.rs
#![allow(clippy::clone_on_copy, unused)]
#![allow(clippy::assigning_clones)]
#![no_main]

extern crate proc_macros;
use proc_macros::with_span;

// lint

struct A(u32);

impl Clone for A {
    fn clone(&self) -> Self {
        //~^ non_canonical_clone_impl
        Self(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        //~^ non_canonical_clone_impl
        source.clone();
        *self = source.clone();
    }
}

impl Copy for A {}

// do not lint

struct B(u32);

impl Clone for B {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for B {}

// do not lint derived (clone's implementation is `*self` here anyway)

#[derive(Clone, Copy)]
struct C(u32);

// do not lint derived (fr this time)

struct D(u32);

#[automatically_derived]
impl Clone for D {
    fn clone(&self) -> Self {
        Self(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        source.clone();
        *self = source.clone();
    }
}

impl Copy for D {}

// do not lint if clone is not manually implemented

struct E(u32);

#[automatically_derived]
impl Clone for E {
    fn clone(&self) -> Self {
        Self(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        source.clone();
        *self = source.clone();
    }
}

impl Copy for E {}

// lint since clone is not derived

#[derive(Copy)]
struct F(u32);

impl Clone for F {
    fn clone(&self) -> Self {
        //~^ non_canonical_clone_impl
        Self(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        //~^ non_canonical_clone_impl
        source.clone();
        *self = source.clone();
    }
}

// do not lint since copy has more restrictive bounds

#[derive(Eq, PartialEq)]
struct Uwu<A: Copy>(A);

impl<A: Copy> Clone for Uwu<A> {
    fn clone(&self) -> Self {
        Self(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        source.clone();
        *self = source.clone();
    }
}

impl<A: std::fmt::Debug + Copy + Clone> Copy for Uwu<A> {}

// should skip proc macros, see https://github.com/rust-lang/rust-clippy/issues/12788
#[derive(proc_macro_derive::NonCanonicalClone)]
pub struct G;

with_span!(
    span

    #[derive(Copy)]
    struct H;
    impl Clone for H {
        fn clone(&self) -> Self {
            todo!()
        }
    }
);

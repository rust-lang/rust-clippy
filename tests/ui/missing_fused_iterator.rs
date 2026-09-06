//@no-rustfix
//@aux-build:proc_macros.rs

#![allow(dead_code)]
#![feature(negative_impls)]
#![warn(clippy::missing_fused_iterator)]

extern crate proc_macros;

use std::iter::{FusedIterator as RenamedFusedIterator, Iterator as RenamedIterator};

pub struct PublicStruct;
//~^ missing_fused_iterator

impl Iterator for PublicStruct {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub enum PublicEnum {
    //~^ missing_fused_iterator
    Empty,
}

impl Iterator for PublicEnum {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub union PublicUnion {
    //~^ missing_fused_iterator
    value: u8,
}

impl Iterator for PublicUnion {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct PublicGeneric<T>(T);
//~^ missing_fused_iterator

impl<T> Iterator for PublicGeneric<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

struct Private;

impl Iterator for Private {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub(crate) struct CrateVisible;

impl Iterator for CrateVisible {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

mod visibility {
    pub struct Reexported;
    //~^ missing_fused_iterator

    impl Iterator for Reexported {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct Returned;
    //~^ missing_fused_iterator

    impl Iterator for Returned {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub fn returned() -> Returned {
        Returned
    }

    pub struct UnreachableInPrivateModule;

    impl Iterator for UnreachableInPrivateModule {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub use visibility::Reexported;

pub fn returned() -> visibility::Returned {
    visibility::returned()
}

pub struct AlreadyFused;

impl Iterator for AlreadyFused {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl std::iter::FusedIterator for AlreadyFused {}

pub struct ConditionallyFused<T>(T);

impl<T> Iterator for ConditionallyFused<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// Deliberately does not cover every `T` for which the `Iterator` implementation applies. Any
// implementation for the nominal type is enough to suppress the lint.
impl<T: Clone> std::iter::FusedIterator for ConditionallyFused<T> {}

pub struct ExplicitlyNotFused;

impl Iterator for ExplicitlyNotFused {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// An explicit negative implementation documents that this type is intentionally not fused.
impl !std::iter::FusedIterator for ExplicitlyNotFused {}

pub struct ExplicitlyNotAnIterator;

// Negative `Iterator` implementations must not satisfy the positive-implementation check.
impl !Iterator for ExplicitlyNotAnIterator {}

#[doc(hidden)]
pub struct DocHidden;
//~^ missing_fused_iterator

impl Iterator for DocHidden {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct IteratorByReference;

// The declared nominal type does not itself implement `Iterator`.
impl Iterator for &IteratorByReference {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

struct OpaqueIterator;

impl Iterator for OpaqueIterator {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// `FusedIterator` would need to be part of this opaque return bound to be exposed publicly.
pub fn opaque_iterator() -> impl Iterator<Item = ()> {
    OpaqueIterator
}

pub struct RenamedTraits;

impl RenamedIterator for RenamedTraits {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl RenamedFusedIterator for RenamedTraits {}

mod shadow_iterator {
    pub trait Iterator {}

    pub struct OnlyLocalIterator;

    impl Iterator for OnlyLocalIterator {}
}

pub use shadow_iterator::OnlyLocalIterator;

mod shadow_fused_iterator {
    pub trait FusedIterator {}

    pub struct RealIterator;
    //~^ missing_fused_iterator

    impl std::iter::Iterator for RealIterator {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl FusedIterator for RealIterator {}
}

pub use shadow_fused_iterator::RealIterator;

macro_rules! local_iterator {
    ($name:ident) => {
        pub struct $name;
        //~^ missing_fused_iterator

        impl Iterator for $name {
            type Item = ();

            fn next(&mut self) -> Option<Self::Item> {
                None
            }
        }
    };
}

local_iterator!(LocalMacroIterator);

proc_macros::external! {
    pub struct ExternalMacroIterator;

    impl Iterator for ExternalMacroIterator {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

#[allow(clippy::missing_fused_iterator)]
pub struct Allowed;

impl Iterator for Allowed {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[expect(clippy::missing_fused_iterator)]
pub struct Expected;

impl Iterator for Expected {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[clippy::msrv = "1.25"]
pub struct BeforeMsrv;

impl Iterator for BeforeMsrv {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[clippy::msrv = "1.26"]
pub struct AtMsrv;
//~^ missing_fused_iterator

impl Iterator for AtMsrv {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

fn main() {}

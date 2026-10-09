//@no-rustfix
//@aux-build:proc_macros.rs

#![expect(dead_code)]
#![feature(negative_impls)]
#![warn(clippy::missing_fused_iterator)]

extern crate proc_macros;

pub mod public_struct {
    pub struct PublicStruct;

    impl Iterator for PublicStruct {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod public_enum {
    pub enum PublicEnum {
        Empty,
    }

    impl Iterator for PublicEnum {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod public_union {
    pub union PublicUnion {
        value: u8,
    }

    impl Iterator for PublicUnion {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod public_generic {
    pub struct PublicGeneric<T>(T);

    impl<T> Iterator for PublicGeneric<T> {
        //~^ missing_fused_iterator
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod private_type {
    struct Private;

    impl Iterator for Private {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod crate_visible_type {
    pub(crate) struct CrateVisible;

    impl Iterator for CrateVisible {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

mod visibility {
    pub struct Reexported;

    impl Iterator for Reexported {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct Returned;

    impl Iterator for Returned {
        //~^ missing_fused_iterator
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

pub mod already_fused {
    pub struct AlreadyFused;

    impl Iterator for AlreadyFused {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl std::iter::FusedIterator for AlreadyFused {}
}

pub mod conditionally_fused {
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
}

pub mod explicitly_not_fused {
    pub struct ExplicitlyNotFused;

    impl Iterator for ExplicitlyNotFused {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    // An explicit negative implementation documents that this type is intentionally not fused.
    impl !std::iter::FusedIterator for ExplicitlyNotFused {}
}

pub mod explicitly_not_an_iterator {
    pub struct ExplicitlyNotAnIterator;

    // Negative `Iterator` implementations must not satisfy the positive-implementation check.
    impl !Iterator for ExplicitlyNotAnIterator {}
}

pub mod doc_hidden {
    #[doc(hidden)]
    pub struct DocHidden;

    impl Iterator for DocHidden {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod iterator_by_reference {
    pub struct IteratorByReference;

    // The declared nominal type does not itself implement `Iterator`.
    impl Iterator for &IteratorByReference {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod opaque {
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
}

pub mod renamed_traits {
    use std::iter::{FusedIterator as RenamedFusedIterator, Iterator as RenamedIterator};

    pub struct RenamedTraits;

    impl RenamedIterator for RenamedTraits {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl RenamedFusedIterator for RenamedTraits {}
}

mod shadow_iterator {
    pub trait Iterator {}

    pub struct OnlyLocalIterator;

    impl Iterator for OnlyLocalIterator {}
}

pub use shadow_iterator::OnlyLocalIterator;

mod shadow_fused_iterator {
    pub trait FusedIterator {}

    pub struct RealIterator;

    impl std::iter::Iterator for RealIterator {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl FusedIterator for RealIterator {}
}

pub use shadow_fused_iterator::RealIterator;

pub mod local_macro {
    macro_rules! local_iterator {
        ($name:ident) => {
            pub struct $name;

            impl Iterator for $name {
                //~^ missing_fused_iterator
                type Item = ();

                fn next(&mut self) -> Option<Self::Item> {
                    None
                }
            }
        };
    }

    local_iterator!(LocalMacroIterator);
}

pub mod external_macro {
    proc_macros::external! {
        pub struct ExternalMacroIterator;

        impl Iterator for ExternalMacroIterator {
            type Item = ();

            fn next(&mut self) -> Option<Self::Item> {
                None
            }
        }
    }
}

pub mod lint_levels {
    pub struct Allowed;

    #[allow(clippy::missing_fused_iterator)]
    impl Iterator for Allowed {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct Expected;

    #[expect(clippy::missing_fused_iterator)]
    impl Iterator for Expected {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod msrv {
    pub struct BeforeMsrv;

    #[clippy::msrv = "1.25"]
    impl Iterator for BeforeMsrv {
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct AtMsrv;

    #[clippy::msrv = "1.26"]
    impl Iterator for AtMsrv {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct AtLintReasons;

    // `#[expect]` and `reason` need 1.81, below that the suppression suggestion uses `#[allow]`.
    #[clippy::msrv = "1.81"]
    impl Iterator for AtLintReasons {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod multiple_impls {
    pub struct Multiple<T>(T);

    impl Iterator for Multiple<u8> {
        //~^ missing_fused_iterator
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl Iterator for Multiple<u16> {
        //~^ missing_fused_iterator
        type Item = u16;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod bounded_generic {
    pub struct Bounded<T>(T);

    //~v missing_fused_iterator
    impl<T: Clone> Iterator for Bounded<T>
    where
        T: Default, // needed for the default value
        T: Copy,
    {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub mod conditional {
    pub struct Conditional;

    #[cfg(not(test))]
    impl Iterator for Conditional {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct ConditionalAttr;

    #[cfg_attr(not(test), cfg(not(test)))]
    impl Iterator for ConditionalAttr {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct InactiveConditionalAttr;

    #[cfg(not(test))]
    #[cfg_attr(test, cfg(any()))]
    impl Iterator for InactiveConditionalAttr {
        //~^ missing_fused_iterator
        type Item = ();

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

fn main() {}

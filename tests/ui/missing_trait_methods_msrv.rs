#![allow(clippy::derive_ord_xor_partial_ord)]
#![warn(clippy::missing_trait_methods)]

#[clippy::msrv = "1.0.0"]
fn msrv0() {
    #[derive(PartialEq, Eq, PartialOrd)]
    struct S {}

    impl Ord for S {
        fn cmp(&self, other: &S) -> std::cmp::Ordering {
            unreachable!()
        }
    }
}

#[clippy::msrv = "1.21.0"]
fn msrv1() {
    #[derive(PartialEq, Eq, PartialOrd)]
    struct S {}

    impl Ord for S {
        //~^ missing_trait_methods
        //~| missing_trait_methods
        fn cmp(&self, other: &S) -> std::cmp::Ordering {
            unreachable!()
        }
    }
}

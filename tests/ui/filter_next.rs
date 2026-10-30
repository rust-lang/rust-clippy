//@aux-build:option_helpers.rs
#![warn(clippy::filter_next)]
#![expect(clippy::disallowed_names)]
#![allow(clippy::useless_vec)]

extern crate option_helpers;

use option_helpers::{IteratorFalsePositives, IteratorMethodFalsePositives};

fn main() {}

fn filter_next() {
    let v = [3, 2, 1, 0, -1, -2, -3];

    // Single-line case.
    let _ = v.iter().filter(|&x| *x < 0).next();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).next_back();
    //~^ filter_next

    // Multi-line case.
    #[rustfmt::skip]
    let _ = v.iter().filter(|&x| {
    //~^ filter_next
                                *x < 0
                            }
                   ).next();

    #[rustfmt::skip]
    let _ = v.iter().filter(|&x| {
    //~^ filter_next
                                *x < 0
                            }
                   ).next_back();

    // Check that we don't lint if the caller is not an `Iterator`.
    let foo = IteratorFalsePositives { foo: 0 };
    let _ = foo.filter().next();

    let foo = IteratorMethodFalsePositives {};
    let _ = foo.filter(42).next();
}

fn filter_next_adapter() {
    let v = [3, 2, 1, 0, -1, -2, -3];

    let _ = v.iter().filter(|&x| *x < 0).map(|x| x * 2).next();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).cloned().next();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).copied().next();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).map(|x| x * 2).next_back();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).cloned().next_back();
    //~^ filter_next

    let _ = v.iter().filter(|&x| *x < 0).copied().next_back();
    //~^ filter_next

    // Multi-line case.
    #[rustfmt::skip]
    let _ = v
    //~^ filter_next
        .iter()
        .filter(|&x| {
            *x < 0
        })
        .map(|x| {
            x * 2
        })
        .next();

    // The result of `find` is used further.
    let _ = v.iter().filter(|&x| *x < 0).map(|x| x * 2).next().unwrap();
    //~^ filter_next

    // Another `filter` is linted by itself, with the first one remaining.
    let _ = v.iter().filter(|&x| *x < 0).filter(|&x| *x < -1).next();
    //~^ filter_next

    // Adapters where `find(..)` followed by the same method on the `Option` is not equivalent.
    let _ = v
        .iter()
        .filter(|&x| *x < 0)
        .filter_map(|x| if *x < -1 { Some(*x) } else { None })
        .next();
    let _ = v.iter().filter(|&x| *x < 0).flat_map(|x| vec![*x]).next();
    let _ = v.iter().filter(|&x| *x < 0).scan(0, |s, x| Some(*s + x)).next();
    let _ = v.iter().filter(|&x| *x < 0).skip(1).next();
    //~^ iter_skip_next
    let _ = v.iter().filter(|&x| *x < 0).enumerate().next();

    // Check that we don't lint if the caller is not an `Iterator`.
    let foo = NotAnIterator;
    let _ = foo.filter().map(|x| x).next();
    let _ = foo.filter().cloned().next();
    let _ = foo.filter().copied().next_back();
}

#[derive(Clone, Copy)]
struct NotAnIterator;

impl NotAnIterator {
    fn filter(self) -> Self {
        self
    }

    fn map(self, _: impl Fn(u32) -> u32) -> Self {
        self
    }

    fn cloned(self) -> Self {
        self
    }

    fn copied(self) -> Self {
        self
    }

    fn next(self) -> Option<u32> {
        None
    }

    fn next_back(self) -> Option<u32> {
        None
    }
}

fn filter_next_back() {
    let v = [3, 2, 1, 0, -1, -2, -3];

    // Check that we don't lint if the caller is not an `Iterator`.
    let foo = IteratorFalsePositives { foo: 0 };
    let _ = foo.filter().next_back();

    let foo = IteratorMethodFalsePositives {};
    let _ = foo.filter(42).next_back();
}

#[clippy::msrv = "1.27"]
fn msrv_1_27() {
    let _ = vec![1].into_iter().filter(|&x| x < 0).next_back();
    //~^ filter_next
}

#[clippy::msrv = "1.26"]
fn msrv_1_26() {
    let _ = vec![1].into_iter().filter(|&x| x < 0).next_back();
}

#[clippy::msrv = "1.27"]
fn msrv_adapter_1_27() {
    let _ = vec![1].into_iter().filter(|&x| x < 0).map(|x| x + 1).next_back();
    //~^ filter_next
}

#[clippy::msrv = "1.26"]
fn msrv_adapter_1_26() {
    let _ = vec![1].into_iter().filter(|&x| x < 0).map(|x| x + 1).next_back();
    let _ = vec![1].into_iter().filter(|&x| x < 0).map(|x| x + 1).next();
    //~^ filter_next
}

mod next_from_other_trait {
    trait MyNext {
        fn next(self) -> u32;
    }
    impl<T: Iterator> MyNext for T {
        fn next(self) -> u32 {
            42
        }
    }

    trait MyNextBack {
        fn next_back(self) -> u32;
    }
    impl<T: DoubleEndedIterator> MyNextBack for T {
        fn next_back(self) -> u32 {
            42
        }
    }

    // Check that we don't lint if `next` and `next_back` are not the ones from `Iterator`.
    fn check() {
        let v = [3, 2, 1, 0, -1, -2, -3];

        let _: u32 = v.iter().filter(|&x| *x < 0).next();
        let _: u32 = v.iter().filter(|&x| *x < 0).map(|x| x * 2).next();
        let _: u32 = v.iter().filter(|&x| *x < 0).next_back();
        let _: u32 = v.iter().filter(|&x| *x < 0).map(|x| x * 2).next_back();
    }
}

mod inherent_filter {
    struct S;

    impl Iterator for S {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            None
        }
    }

    impl DoubleEndedIterator for S {
        fn next_back(&mut self) -> Option<u32> {
            None
        }
    }

    impl S {
        // shadows `Iterator::filter`
        fn filter(self, _: u32) -> S {
            self
        }
    }

    // Check that we don't lint if `filter` is not the one from `Iterator`.
    fn check() {
        let _ = S.filter(0).next();
        let _ = S.filter(0).next_back();
        let _ = S.filter(0).map(|x| x + 1).next();
        let _ = S.filter(0).map(|x| x + 1).next_back();
    }
}

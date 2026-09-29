#![warn(clippy::assert_is_empty)]

use std::collections::{BinaryHeap, VecDeque};

// Don't lint: `BinaryHeap::as_slice` was stabilized in 1.80, so the rewrite does
// not compile under this MSRV.
#[clippy::msrv = "1.79"]
fn below_msrv() {
    let binary_heap = BinaryHeap::<i32>::new();
    assert!(binary_heap.is_empty());
    assert!(!binary_heap.is_empty());

    // The other collections need no minimum version.
    let vec_deque = VecDeque::<i32>::new();
    assert!(vec_deque.is_empty());
    //~^ assert_is_empty
}

#[clippy::msrv = "1.80"]
fn at_msrv() {
    let binary_heap = BinaryHeap::<i32>::new();
    assert!(binary_heap.is_empty());
    //~^ assert_is_empty
}

fn main() {
    below_msrv();
    at_msrv();
}

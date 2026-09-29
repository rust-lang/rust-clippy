#![warn(clippy::assert_is_empty)]
#![allow(clippy::useless_vec)]

// Element types that cannot be written at the call site fall back to a bare `[]`,
// which is not guaranteed to infer, so the suggestion is `MaybeIncorrect`. Element
// types that can be written anywhere keep the typed empty array.

use std::collections::BinaryHeap;
use std::fmt::Debug;

fn opaque_array() -> [impl PartialEq + Debug; 1] {
    [1]
}

fn opaque_vec() -> Vec<impl PartialEq + Debug> {
    vec![1]
}

fn opaque_heap() -> BinaryHeap<impl Ord + Debug> {
    BinaryHeap::from([1])
}

fn opaque() {
    // `impl Trait` is forbidden in cast types.
    let array = opaque_array();
    assert!(array.is_empty());
    //~^ assert_is_empty

    let vec = opaque_vec();
    assert!(vec.is_empty());
    //~^ assert_is_empty
    let vec_ref = &vec;
    assert!(vec_ref.is_empty());
    //~^ assert_is_empty

    let heap = opaque_heap();
    assert!(heap.is_empty());
    //~^ assert_is_empty
}

fn argument_position_impl_trait(array: &[impl PartialEq + Debug; 1]) {
    assert!(array.is_empty());
    //~^ assert_is_empty
}

fn generic<T: PartialEq + Debug>(array: &[T; 1]) {
    // A named generic parameter is in scope, so the typed array is kept.
    assert!(array.is_empty());
    //~^ assert_is_empty
}

fn function_local() {
    // Prints as `function_local::Local`, which does not resolve.
    #[derive(Debug, PartialEq)]
    struct Local;
    let vec = vec![Local];
    assert!(vec.is_empty());
    //~^ assert_is_empty
    let array = [Local];
    assert!(array.is_empty());
    //~^ assert_is_empty
}

mod outer {
    mod hidden {
        #[derive(Debug, PartialEq)]
        pub struct Reexported;
    }
    pub use hidden::Reexported;
}

fn private_module() {
    // Prints through its defining module `outer::hidden`, which is private here.
    let vec = vec![outer::Reexported];
    assert!(vec.is_empty());
    //~^ assert_is_empty
}

#[derive(Debug, PartialEq)]
struct CrateRoot;

mod submodule {
    pub fn not_imported() {
        // Prints as `CrateRoot`, which is not in scope in this module.
        let vec: Vec<super::CrateRoot> = vec![super::CrateRoot];
        assert!(vec.is_empty());
        //~^ assert_is_empty
    }
}

fn main() {
    opaque();
    argument_position_impl_trait(&[1]);
    generic(&[1]);
    function_local();
    private_module();
    submodule::not_imported();
}

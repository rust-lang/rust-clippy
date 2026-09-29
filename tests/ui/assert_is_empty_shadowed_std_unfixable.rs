//@no-rustfix
#![warn(clippy::assert_is_empty)]
#![allow(non_camel_case_types, clippy::const_is_empty)]

// Printed element types begin with `std` or a primitive type name. Where the crate
// rebinds one of those names, the typed empty array may not resolve, so these
// suggestions are `MaybeIncorrect`. The ones below do not compile.

mod shadowed {
    mod std {}

    pub fn shadowed_std() {
        let heap = ::std::collections::BinaryHeap::<String>::new();
        assert!(heap.is_empty());
        //~^ assert_is_empty

        let array = [String::new()];
        assert!(array.is_empty());
        //~^ assert_is_empty

        let nested: Vec<::std::collections::HashMap<String, i32>> = Vec::new();
        assert!(nested.is_empty());
        //~^ assert_is_empty
    }
}

#[derive(Debug, PartialEq)]
struct i32;

fn shadowed_primitive() {
    let array: [::core::primitive::i32; 1] = [1];
    assert!(array.is_empty());
    //~^ assert_is_empty
}

fn main() {
    shadowed::shadowed_std();
    shadowed_primitive();
}

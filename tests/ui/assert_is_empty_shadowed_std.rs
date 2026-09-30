#![warn(clippy::assert_is_empty)]

// The collection constructor is named by a path beginning with `::`, so it still
// resolves where a local item shadows `std`. Element types that do not print
// through `std` are unaffected. See `assert_is_empty_shadowed_std_unfixable.rs`
// for element types that do.
mod shadowed {
    mod std {}

    pub fn shadowed_std() {
        let map = ::std::collections::HashMap::<i32, i32>::new();
        assert!(map.is_empty());
        //~^ assert_is_empty
        let heap = ::std::collections::BinaryHeap::<i32>::new();
        assert!(heap.is_empty());
        //~^ assert_is_empty
    }
}

fn main() {
    shadowed::shadowed_std();
}

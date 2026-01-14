//@compile-flags: -Zdeduplicate-diagnostics=yes
//@aux-build: proc_macros.rs

#![warn(clippy::indexing_slicing)]
// We also check the out_of_bounds_indexing lint here, because it lints similar things and
// we want to avoid false positives.
#![warn(clippy::out_of_bounds_indexing)]
#![allow(
    unconditional_panic,
    clippy::no_effect,
    clippy::unnecessary_operation,
    clippy::useless_vec
)]

extern crate proc_macros;
use proc_macros::with_span;

const ARR: [i32; 2] = [1, 2];
const REF: &i32 = &ARR[idx()]; // This should be linted, since `suppress-restriction-lint-in-const` default is false.
//~^ ERROR: indexing may panic

const fn idx() -> usize {
    1
}
const fn idx4() -> usize {
    4
}

with_span!(
    span

    fn dont_lint_proc_macro_array() {
        let x = [1, 2, 3, 4];
        let index: usize = 1;
        x[index];
        x[10];

        let x = vec![0; 5];
        let index: usize = 1;
        x[index];
        x[10];
    }
);

fn main() {
    let x = [1, 2, 3, 4];
    let index: usize = 1;
    x[index];
    //~^ ERROR: indexing may panic
    // Ok, let rustc's `unconditional_panic` lint handle `usize` indexing on arrays.
    x[4];
    //~^ out_of_bounds_indexing
    // Ok, let rustc's `unconditional_panic` lint handle `usize` indexing on arrays.
    x[1 << 3];
    //~^ out_of_bounds_indexing

    // Ok, should not produce stderr.
    x[0];
    // Ok, should not produce stderr.
    x[3];
    // Ok, should not produce stderr.
    x[const { idx() }];
    // Ok, let rustc's `unconditional_panic` lint handle `usize` indexing on arrays.
    x[const { idx4() }];
    // This should be linted, since `suppress-restriction-lint-in-const` default is false.
    const { &ARR[idx()] };
    //~^ ERROR: indexing may panic
    // This should be linted, since `suppress-restriction-lint-in-const` default is false.
    const { &ARR[idx4()] };
    //~^ ERROR: indexing may panic

    let y = &x;
    // Ok, referencing shouldn't affect this lint. See the issue 6021
    y[0];
    // Ok, rustc will handle references too.
    y[4];
    //~^ out_of_bounds_indexing

    let v = vec![0; 5];
    v[0];
    //~^ ERROR: indexing may panic
    v[10];
    //~^ ERROR: indexing may panic
    v[1 << 3];
    //~^ ERROR: indexing may panic

    // Out of bounds
    const N: usize = 15;
    // In bounds
    const M: usize = 3;
    // Ok, let rustc's `unconditional_panic` lint handle `usize` indexing on arrays.
    x[N];
    //~^ out_of_bounds_indexing
    // Ok, should not produce stderr.
    x[M];
    v[N];
    //~^ ERROR: indexing may panic
    v[M];
    //~^ ERROR: indexing may panic

    let slice = &x;
    let _ = x[4];
    //~^ out_of_bounds_indexing
}

mod issue_16384 {
    use std::ops::{Deref, Index};

    enum Key {
        A,
        B,
    }

    struct MyMap {
        data: [&'static str; 2],
    }

    impl Index<Key> for MyMap {
        type Output = &'static str;
        fn index(&self, key: Key) -> &Self::Output {
            match key {
                Key::A => &self.data[0],
                Key::B => &self.data[1],
            }
        }
    }

    impl Deref for MyMap {
        type Target = [&'static str];
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    fn test() {
        let map = MyMap { data: ["a", "b"] };
        // Should NOT lint: `get()` takes `usize`, not `Key`
        let _ = map[Key::A];
    }
}

mod custom_range_index {
    use std::ops::{Index, Range};

    struct MyVec<T>(T);

    impl<T> Index<Range<u32>> for MyVec<T> {
        type Output = T;
        fn index(&self, _index: Range<u32>) -> &Self::Output {
            &self.0
        }
    }

    impl<T> MyVec<T> {
        fn get(&self, _index: Range<u32>) -> Option<&T> {
            Some(&self.0)
        }
    }

    fn test() {
        let v = MyVec(1);
        // Should lint: `get(Range<u32>)` matches index type
        let _ = v[0..1];
        //~^ indexing_slicing
    }
}

mod range_variable {
    fn test() {
        let arr = [1, 2, 3, 4, 5];
        // Should lint: higher::Range::hir() misses this
        let r = 1..3;
        let _ = &arr[r];
        //~^ indexing_slicing
    }
}

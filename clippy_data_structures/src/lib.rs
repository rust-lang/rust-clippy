#![feature(
    array_windows,
    cmp_minmax,
    if_let_guard,
    maybe_uninit_slice,
    min_specialization,
    new_range_api,
    rustc_private,
    slice_partition_dedup
)]
#![allow(
    clippy::missing_docs_in_private_items,
    clippy::must_use_candidate,
    rustc::diagnostic_outside_of_impl,
    rustc::untranslatable_diagnostic,
    clippy::literal_string_with_formatting_args
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications,
    rustc::internal
)]

extern crate rustc_arena;
#[expect(unused_extern_crates, reason = "needed for tests to link to librustcdriver")]
extern crate rustc_driver;
extern crate rustc_index;

use core::ops::RangeBounds;

mod range;
mod sorted;
mod traits;

pub mod bit_slice;
pub use bit_slice::BitSlice;

pub mod bit_set_2d;
pub use bit_set_2d::{BitSlice2d, GrowableBitSet2d};

mod slice_set;
pub use slice_set::SliceSet;

/// An iterator where the size hint is provided by calling `Iterator::count`.
pub struct CountedIter<T>(pub T);
impl<T> Iterator for CountedIter<T>
where
    T: Iterator + Clone,
{
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }
    fn count(self) -> usize {
        self.0.count()
    }
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.0.fold(init, f)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.0.clone().count();
        (size, Some(size))
    }
}

/// Moves items within the slice leaving behind the default value at indices from the source range
/// which are not also part of the destination range.
#[inline]
pub fn move_within_slice(
    slice: &mut [impl Copy + Default],
    src: impl Clone + RangeBounds<usize> + range::Len + range::SubtractFromEdge,
    dst: usize,
) {
    slice.copy_within(src.clone(), dst);
    let src_len = src.len();
    for x in &mut slice[src.subtract_from_edge(dst..dst + src_len)] {
        *x = Default::default();
    }
}

#[test]
fn test_move_within_slice() {
    let slice = &mut [0, 1, 2, 3, 4];
    move_within_slice(slice, 0..2, 2);
    assert_eq!(slice, &[0, 0, 0, 1, 4]);
    move_within_slice(slice, 3..5, 3);
    assert_eq!(slice, &[0, 0, 0, 1, 4]);
    move_within_slice(slice, 3..5, 2);
    assert_eq!(slice, &[0, 0, 1, 4, 0]);
    move_within_slice(slice, 2..4, 3);
    assert_eq!(slice, &[0, 0, 0, 1, 4]);
}

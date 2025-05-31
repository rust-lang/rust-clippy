#![feature(array_windows)]
#![feature(if_let_guard)]
#![feature(min_specialization)]
#![feature(new_range_api)]
#![feature(rustc_private)]
#![feature(slice_partition_dedup)]

extern crate rustc_arena;
extern crate rustc_index;
extern crate rustc_mir_dataflow;

use crate::traits::{RangeLen, SubtractRangeItemsFromEdge};
use arrayvec::ArrayVec;
use core::ops::RangeBounds;
use smallvec::SmallVec;

mod sorted;
mod traits;

pub mod bit_slice;
pub use bit_slice::BitSlice;

pub mod bit_set_2d;
pub use bit_set_2d::{BitSlice2d, GrowableBitSet2d};

mod slice_set;
pub use slice_set::SliceSet;

mod vec_set;
pub type VecSet<T> = vec_set::VecSet<Vec<T>>;
pub type SmallVecSet<T, const N: usize> = vec_set::VecSet<SmallVec<[T; N]>>;
pub type ArrayVecSet<T, const N: usize> = vec_set::VecSet<ArrayVec<T, N>>;

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
    src: impl Clone + RangeBounds<usize> + RangeLen + SubtractRangeItemsFromEdge,
    dst: usize,
) {
    slice.copy_within(src.clone(), dst);
    let src_len = src.len();
    for x in &mut slice[src.subtract_range_items_from_edge(dst..dst + src_len)] {
        *x = Default::default()
    }
}

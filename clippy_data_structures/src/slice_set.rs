use crate::sorted;
use crate::traits::SortedIndex;
use core::borrow::Borrow;
use core::mem::{MaybeUninit, transmute};
use core::ops::Deref;
use core::{iter, slice};
use rustc_arena::DroplessArena;

/// A wrapper around a slice where all items are unique and sorted.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SliceSet<T> {
    data: [T],
}
impl<T> SliceSet<T> {
    /// Gets an empty set.
    #[inline]
    #[must_use]
    pub const fn empty<'a>() -> &'a Self {
        Self::from_sorted_unchecked(&[])
    }

    /// Interprets the reference as a set containing a single item.
    #[inline]
    #[must_use]
    pub const fn from_ref(value: &T) -> &Self {
        Self::from_sorted_unchecked(slice::from_ref(value))
    }

    /// Same as `from_sorted`, but without debug assertions.
    #[inline]
    pub(crate) const fn from_sorted_unchecked(slice: &[T]) -> &Self {
        // SAFETY: `SliceSet<T>`` is a transparent wrapper around `T`.
        unsafe { transmute::<&[T], &SliceSet<T>>(slice) }
    }

    /// Gets the current set as a regular slice.
    #[inline]
    #[must_use]
    pub const fn as_raw_slice(&self) -> &[T] {
        &self.data
    }

    /// Checks if the set contains the given value.
    #[inline]
    #[must_use]
    pub fn contains<Q>(&self, item: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data.binary_search_by(|x| x.borrow().cmp(item)).is_ok()
    }

    /// Gets the specified item from the set. Returns `None` if it doesn't exist.
    #[inline]
    #[must_use]
    pub fn get<Q>(&self, item: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data
            .binary_search_by(|x| x.borrow().cmp(item))
            .ok()
            .map(|i| &self.data[i])
    }

    /// Gets the index of the specified item in the set. Returns `None` if it doesn't exist.
    #[inline]
    #[must_use]
    pub fn get_index<Q>(&self, item: &Q) -> Option<usize>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data.binary_search_by(|x| x.borrow().cmp(item)).ok()
    }

    /// Gets a subset of the current set.
    #[inline]
    #[must_use]
    pub fn get_range<Q>(&self, range: impl SortedIndex<T, Q>) -> &Self
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        Self::from_sorted_unchecked(
            &self.data[range.find_range(&self.data, |slice, target| {
                slice.binary_search_by(|x| x.borrow().cmp(target))
            })],
        )
    }
}
impl<T: Ord> SliceSet<T> {
    /// Assumes the given slice is sorted with no duplicates.
    ///
    /// Will panic with debug assertions enabled if the given slice is unsorted or contains
    /// duplicates.
    #[inline]
    #[must_use]
    pub fn from_sorted(slice: &[T]) -> &Self {
        debug_assert!(sorted::is_slice_set(slice));
        Self::from_sorted_unchecked(slice)
    }

    /// Sorts the given slice and assumes no duplicates.
    ///
    /// Will panic with debug assertions enabled if the given slice contains duplicates.
    #[inline]
    #[must_use]
    pub fn from_unsorted_slice(slice: &mut [T]) -> &Self {
        slice.sort_unstable();
        Self::from_sorted(slice)
    }

    /// Sorts and partitions out duplicates from the given slice.
    #[inline]
    #[must_use]
    pub fn from_unsorted_slice_dedup(slice: &mut [T]) -> &Self {
        slice.sort_unstable();
        Self::from_sorted_unchecked(slice.partition_dedup().0)
    }

    /// Checks if this set is a subset of another.
    #[inline]
    #[must_use]
    pub fn is_subset_of(&self, other: &Self) -> bool {
        if self.len() > other.len() {
            return false;
        }
        if sorted::should_binary_search(other.len(), self.len()) {
            sorted::is_subset_of_binary(self, other)
        } else {
            sorted::is_subset_of_linear(self, other)
        }
    }

    /// Checks if this set is a superset of another.
    #[inline]
    #[must_use]
    pub fn is_superset_of(&self, other: &Self) -> bool {
        other.is_subset_of(self)
    }
}
impl<T: Ord + Copy> SliceSet<T> {
    /// Creates a new set allocated into an arena which is the union of two sorted lists.
    ///
    /// # Panics
    /// Panics if either iterator returns more than their `len` functions indicate.
    #[inline]
    #[must_use]
    pub fn from_sorted_union_into_arena(
        arena: &DroplessArena,
        xs: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
        ys: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> &Self {
        let xs = xs.into_iter();
        let ys = ys.into_iter();
        let len = xs.len().checked_add(ys.len()).unwrap();
        if len == 0 {
            Self::empty()
        } else {
            Self::from_sorted(sorted::union_fill_uninit(
                arena.alloc_from_iter(iter::repeat_with(|| MaybeUninit::uninit()).take(len)),
                xs,
                ys,
                Ord::cmp,
            ))
        }
    }
}

impl<T> Deref for SliceSet<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T> Borrow<[T]> for SliceSet<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        &self.data
    }
}

impl<'a, T> IntoIterator for &'a SliceSet<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

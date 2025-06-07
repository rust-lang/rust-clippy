use crate::sorted;
use crate::traits::SortedIndex;
use core::borrow::Borrow;
use core::mem::transmute;
use core::ops::Deref;
use core::slice;

/// Wrapper type around a `Vec`-like or slice type where all items are unique and sorted.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SliceSet<T> {
    data: [T],
}
impl<T> SliceSet<T> {
    #[inline]
    pub const fn empty() -> &'static Self {
        Self::from_sorted_unchecked(&[])
    }

    #[inline]
    pub const fn from_ref(value: &T) -> &Self {
        Self::from_sorted_unchecked(slice::from_ref(value))
    }

    /// Same as `from_sorted`, but without debug assertions.
    #[inline]
    pub(crate) const fn from_sorted_unchecked(slice: &[T]) -> &Self {
        // SAFETY: `SliceSet<T>`` is a transparent wrapper around `T`.
        unsafe { transmute::<&[T], &SliceSet<T>>(slice) }
    }

    /// Gets the current value as a regular slice.
    #[inline]
    pub const fn as_raw_slice(&self) -> &[T] {
        &self.data
    }

    /// Checks if the set contains the given value.
    #[inline]
    pub fn contains<Q>(&self, item: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data.binary_search_by(|x| x.borrow().cmp(item)).is_ok()
    }

    /// Gets the specified item from the set.
    #[inline]
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

    /// Gets the index of the specified item in the set.
    #[inline]
    pub fn get_index<Q>(&self, item: &Q) -> Option<usize>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data.binary_search_by(|x| x.borrow().cmp(item)).ok()
    }

    /// Gets a subset of the current set.
    #[inline]
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
    pub fn from_sorted(slice: &[T]) -> &Self {
        debug_assert!(sorted::is_slice_set(slice));
        Self::from_sorted_unchecked(slice)
    }

    /// Sorts the given slice and assumes no duplicates.
    ///
    /// Will panic with debug assertions enabled if the given slice contains duplicates.
    #[inline]
    pub fn from_unsorted_slice(slice: &mut [T]) -> &Self {
        slice.sort_unstable();
        Self::from_sorted(slice)
    }

    /// Sorts and partitions out duplicates from the given slice.
    #[inline]
    pub fn from_unsorted_slice_dedup(slice: &mut [T]) -> &Self {
        slice.sort_unstable();
        Self::from_sorted_unchecked(slice.partition_dedup().0)
    }

    /// Checks if this set is a subset of another set.
    #[inline]
    pub fn is_subset_of<U: ?Sized + Borrow<Self>>(&self, other: &U) -> bool {
        let other: &Self = other.borrow();
        if self.len() > other.len() {
            return false;
        }
        if <U as sorted::ShouldBinarySearchSpec>::should_binary_search(other.len(), self.len()) {
            sorted::is_subset_of_binary(self, other)
        } else {
            sorted::is_subset_of_linear(self, other)
        }
    }

    #[inline]
    pub fn is_superset_of(&self, other: &Self) -> bool {
        other.is_subset_of(self)
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

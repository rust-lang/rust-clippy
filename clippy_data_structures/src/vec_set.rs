use crate::traits::{SortedIndex, VecLike, VecLikeCapacity, VecLikeDedup};
use crate::{SliceSet, sorted};
use arrayvec::ArrayVec;
use core::borrow::Borrow;
use core::ops::Deref;
use core::{mem, slice};
use smallvec::SmallVec;

trait FindSpec: VecLike {
    fn find<Q>(list: &[Self::Item], item: &Q) -> Result<usize, usize>
    where
        Self::Item: Borrow<Q>,
        Q: ?Sized + Ord;
}
impl<T: VecLike> FindSpec for T {
    #[inline]
    default fn find<Q>(list: &[Self::Item], item: &Q) -> Result<usize, usize>
    where
        Self::Item: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        list.binary_search_by(|x| x.borrow().cmp(item))
    }
}
impl<T, const N: usize> FindSpec for ArrayVec<T, N> {
    #[inline]
    fn find<Q>(list: &[Self::Item], item: &Q) -> Result<usize, usize>
    where
        Self::Item: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if N <= 6 {
            sorted::linear_search_by(list, |x| x.borrow().cmp(item))
        } else {
            list.binary_search_by(|x| x.borrow().cmp(item))
        }
    }
}

/// Wrapper type around a `Vec`-like type where all items are unique and sorted.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VecSet<T> {
    data: T,
}
impl<T> VecSet<Vec<T>> {
    #[inline]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }
}
impl<T, const N: usize> VecSet<SmallVec<[T; N]>> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: SmallVec::new_const(),
        }
    }
}
impl<T, const N: usize> VecSet<ArrayVec<T, N>> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: ArrayVec::new_const(),
        }
    }
}
impl<T: VecLikeCapacity> VecSet<T> {
    /// Creates a new, empty vec with the given capacity.
    #[inline]
    pub fn with_capacity(size: usize) -> Self {
        Self {
            data: T::with_capacity(size),
        }
    }

    /// Reserves space for at least `additional` more items.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }
}
impl<T: VecLike<Item: Ord>> VecSet<T> {
    /// Assumes the given slice is sorted with no duplicates.
    ///
    /// Will panic with debug assertions enabled if the given slice is unsorted or contains
    /// duplicates.
    #[inline]
    pub fn from_sorted(data: T) -> Self {
        debug_assert!(sorted::is_slice_set(data.borrow()));
        Self { data }
    }

    /// Sorts the given slice and assumes no duplicates.
    ///
    /// Will panic with debug assertions enabled if the given slice contains duplicates.
    #[inline]
    pub fn from_unsorted(mut data: T) -> Self {
        data.borrow_mut().sort();
        debug_assert!(sorted::is_slice_set(data.borrow()));
        Self { data }
    }

    /// Inserts the given item into the set.
    ///
    /// If the item already exists in the set, it will be replaced by the new item and returned.
    /// Otherwise this will return `None`.
    pub fn insert(&mut self, item: T::Item) -> Option<T::Item> {
        match <T as FindSpec>::find(self.data.borrow(), &item) {
            Ok(i) => Some(mem::replace(&mut self.data.borrow_mut()[i], item)),
            Err(i) => {
                self.data.insert(i, item);
                None
            },
        }
    }

    /// Inserts the given item into the set if there is sufficient capacity to do so.
    ///
    /// If the item already exists in the set, it will be replaced by the new item and returned.
    /// Otherwise this will return `None`.
    pub fn insert_within_capacity(&mut self, item: T::Item) -> Result<Option<T::Item>, T::Item> {
        match <T as FindSpec>::find(self.data.borrow(), &item) {
            Ok(i) => Ok(Some(mem::replace(&mut self.data.borrow_mut()[i], item))),
            Err(i) => self.data.insert_within_capacity(i, item).map(|()| None),
        }
    }

    #[inline]
    pub fn is_superset_of(&self, other: &SliceSet<T::Item>) -> bool {
        other.is_subset_of(self)
    }
}
impl<T: VecLike> VecSet<T> {
    /// Checks if the set contains the given value.
    #[inline]
    pub fn contains<Q>(&self, item: &Q) -> bool
    where
        T::Item: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        <T as FindSpec>::find(self.data.borrow(), item).is_ok()
    }

    /// Gets the specified item from the set.
    #[inline]
    pub fn get<Q>(&self, item: &Q) -> Option<&T::Item>
    where
        T::Item: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match <T as FindSpec>::find(self.data.borrow(), item) {
            Ok(i) => Some(&self.data.borrow()[i]),
            Err(_) => None,
        }
    }

    /// Gets a subset of the current set which .
    #[inline]
    pub fn get_range<Q>(&self, range: impl SortedIndex<T::Item, Q>) -> &SliceSet<T::Item>
    where
        T::Item: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        SliceSet::from_sorted_unchecked(
            &self.data.borrow()[range.find_range(&self.data.borrow(), |list, item| <T as FindSpec>::find(list, item))],
        )
    }

    /// Removes all items from the set.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    #[inline]
    pub fn drain<Q>(&mut self, range: impl SortedIndex<T::Item, Q>) -> T::Drain<'_>
    where
        T::Item: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.data
            .drain(range.find_range(&self.data.borrow(), |list, item| <T as FindSpec>::find(list, item)))
    }

    /// Removes the given item from the set. Returns `None` if the set does not contain the item.
    pub fn remove<Q>(&mut self, item: &Q) -> Option<T::Item>
    where
        T::Item: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match <T as FindSpec>::find(self.data.borrow(), item) {
            Ok(i) => Some(self.data.remove(i)),
            Err(_) => None,
        }
    }

    /// Retains only the items for which the given predicate returns `true`.
    #[inline]
    pub fn retain(&mut self, f: impl FnMut(&mut T::Item) -> bool) {
        self.data.retain(f);
    }
}
impl<T: VecLikeCapacity<Item: Ord> + Extend<T::Item>> VecSet<T> {
    /// Replaces the contents of this set with the union of two sets.
    #[inline]
    pub fn replace_with_union(&mut self, xs: impl IntoIterator<Item = T::Item>, ys: impl IntoIterator<Item = T::Item>) {
        self.clear();
        let xs = xs.into_iter();
        let ys = ys.into_iter();
        self.reserve(xs.size_hint().0 + ys.size_hint().0);
        sorted::fill_empty_from_iter_union(&mut self.data, xs, ys, |x, y| x.cmp(y));
        debug_assert!(sorted::is_slice_set(self.data.borrow()));
    }
}

impl<T: VecLikeDedup<Item: Ord>> VecSet<T> {
    /// Sorts and removes duplicates from the given vec.
    #[inline]
    pub fn from_unsorted_dedup(mut data: T) -> Self {
        data.borrow_mut().sort();
        data.dedup();
        Self { data }
    }
}

impl<T: VecLike<Item: Ord> + FromIterator<T::Item>> VecSet<T> {
    #[inline]
    pub fn from_sorted_iter(iter: impl IntoIterator<Item = T::Item>) -> Self {
        Self::from_sorted(T::from_iter(iter))
    }

    #[inline]
    pub fn from_unsorted_iter(iter: impl IntoIterator<Item = T::Item>) -> Self {
        Self::from_unsorted(T::from_iter(iter))
    }
}
impl<T: VecLikeDedup<Item: Ord> + FromIterator<T::Item>> VecSet<T> {
    #[inline]
    pub fn from_unsorted_iter_dedup(iter: impl IntoIterator<Item = T::Item>) -> Self {
        Self::from_unsorted_dedup(T::from_iter(iter))
    }
}

impl<T: VecLikeCapacity<Item: Ord> + Extend<T::Item>> VecSet<T> {
    #[inline]
    pub fn extend_sorted(&mut self, iter: impl IntoIterator<Item = T::Item>) {
        sorted::union(&mut self.data, iter.into_iter(), |x, y| x.cmp(y), |x, y| *x = y);
        debug_assert!(sorted::is_slice_set(self.data.borrow()));
    }
}

impl<T: VecLike> Deref for VecSet<T> {
    type Target = SliceSet<T::Item>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        SliceSet::from_sorted_unchecked(self.data.borrow())
    }
}
impl<T: VecLike> Borrow<SliceSet<T::Item>> for VecSet<T> {
    #[inline]
    fn borrow(&self) -> &SliceSet<T::Item> {
        SliceSet::from_sorted_unchecked(self.data.borrow())
    }
}
impl<T: VecLike> Borrow<[T::Item]> for VecSet<T> {
    #[inline]
    fn borrow(&self) -> &[T::Item] {
        self.data.borrow()
    }
}

impl<T> IntoIterator for VecSet<T>
where
    T: VecLike + IntoIterator<Item = <T as VecLike>::Item>,
{
    type Item = <T as IntoIterator>::Item;
    type IntoIter = <T as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
impl<'a, T: VecLike> IntoIterator for &'a VecSet<T> {
    type Item = &'a T::Item;
    type IntoIter = slice::Iter<'a, T::Item>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.borrow().iter()
    }
}

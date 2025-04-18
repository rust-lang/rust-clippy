use arrayvec::ArrayVec;
use core::borrow::BorrowMut;
use core::ops::{Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::range;
use core::slice::SliceIndex;
use smallvec::SmallVec;

/// Trait for types which act like a `Vec`.
pub trait VecLike: BorrowMut<[Self::Item]> {
    type Item;
    type Drain<'a>
    where
        Self: 'a,
        Self::Item: 'a;

    fn clear(&mut self);
    fn drain(&mut self, range: impl RangeBounds<usize>) -> Self::Drain<'_>;
    fn push(&mut self, item: Self::Item);
    fn insert(&mut self, idx: usize, item: Self::Item);
    fn remove(&mut self, idx: usize) -> Self::Item;
    fn retain(&mut self, f: impl FnMut(&mut Self::Item) -> bool);
    fn splice(&mut self, range: impl RangeBounds<usize>, replacement: impl IntoIterator<Item = Self::Item>);
    fn insert_within_capacity(&mut self, idx: usize, item: Self::Item) -> Result<(), Self::Item>;
}
pub trait VecLikeCapacity: VecLike {
    /// Creates a new value with the specified capacity.
    fn with_capacity(size: usize) -> Self;

    /// Reserves space for at least `additional` more items.
    fn reserve(&mut self, additional: usize);
}
pub trait VecLikeDedup: VecLike {
    /// Removes consecutive repeated elements from the vector.
    fn dedup(&mut self);
}

impl<T> VecLike for Vec<T> {
    type Item = T;
    type Drain<'a>
        = std::vec::Drain<'a, T>
    where
        Self: 'a,
        Self::Item: 'a;

    #[inline]
    fn clear(&mut self) {
        self.clear();
    }

    #[inline]
    fn drain(&mut self, range: impl RangeBounds<usize>) -> Self::Drain<'_> {
        self.drain(range)
    }

    #[inline]
    fn push(&mut self, item: Self::Item) {
        self.push(item);
    }

    #[inline]
    #[track_caller]
    fn insert(&mut self, idx: usize, item: T) {
        self.insert(idx, item);
    }

    #[inline]
    #[track_caller]
    fn insert_within_capacity(&mut self, idx: usize, item: T) -> Result<(), T> {
        if self.len() < self.capacity() {
            self.insert(idx, item);
            Ok(())
        } else {
            Err(item)
        }
    }

    #[inline]
    #[track_caller]
    fn remove(&mut self, idx: usize) -> T {
        self.remove(idx)
    }

    #[inline]
    fn retain(&mut self, f: impl FnMut(&mut T) -> bool) {
        self.retain_mut(f);
    }

    #[inline]
    #[track_caller]
    fn splice(&mut self, range: impl RangeBounds<usize>, replacement: impl IntoIterator<Item = Self::Item>) {
        self.splice(range, replacement);
    }
}
impl<T> VecLikeCapacity for Vec<T> {
    #[inline]
    fn with_capacity(size: usize) -> Self {
        Self::with_capacity(size)
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}
impl<T: PartialEq> VecLikeDedup for Vec<T> {
    #[inline]
    fn dedup(&mut self) {
        self.dedup();
    }
}

impl<T, const N: usize> VecLike for SmallVec<[T; N]> {
    type Item = T;
    type Drain<'a>
        = smallvec::Drain<'a, [T; N]>
    where
        Self: 'a,
        Self::Item: 'a;

    #[inline]
    fn clear(&mut self) {
        self.clear();
    }

    #[inline]
    fn drain(&mut self, range: impl RangeBounds<usize>) -> Self::Drain<'_> {
        self.drain(range)
    }

    #[inline]
    fn push(&mut self, item: Self::Item) {
        self.push(item);
    }

    #[inline]
    #[track_caller]
    fn insert(&mut self, idx: usize, item: T) {
        self.insert(idx, item);
    }

    #[inline]
    #[track_caller]
    fn insert_within_capacity(&mut self, idx: usize, item: T) -> Result<(), T> {
        if self.len() < self.capacity() {
            self.insert(idx, item);
            Ok(())
        } else {
            Err(item)
        }
    }

    #[inline]
    #[track_caller]
    fn remove(&mut self, idx: usize) -> T {
        self.remove(idx)
    }

    #[inline]
    fn retain(&mut self, f: impl FnMut(&mut T) -> bool) {
        self.retain(f);
    }

    #[inline]
    #[track_caller]
    fn splice(&mut self, range: impl RangeBounds<usize>, replacement: impl IntoIterator<Item = Self::Item>) {
        let i = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&x) => x,
            Bound::Excluded(&x) => x + 1,
        };
        self.drain(range);
        self.insert_many(i, replacement);
    }
}
impl<T, const N: usize> VecLikeCapacity for SmallVec<[T; N]> {
    #[inline]
    fn with_capacity(size: usize) -> Self {
        Self::with_capacity(size)
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}
impl<T: PartialEq, const N: usize> VecLikeDedup for SmallVec<[T; N]> {
    #[inline]
    fn dedup(&mut self) {
        self.dedup();
    }
}

impl<T, const N: usize> VecLike for ArrayVec<T, N> {
    type Item = T;
    type Drain<'a>
        = arrayvec::Drain<'a, T, N>
    where
        Self: 'a,
        Self::Item: 'a;

    #[inline]
    fn clear(&mut self) {
        self.clear();
    }

    #[inline]
    fn drain(&mut self, range: impl RangeBounds<usize>) -> Self::Drain<'_> {
        self.drain(range)
    }

    #[inline]
    fn push(&mut self, item: Self::Item) {
        self.push(item);
    }

    #[inline]
    #[track_caller]
    fn insert(&mut self, idx: usize, item: T) {
        self.insert(idx, item);
    }

    #[inline]
    fn insert_within_capacity(&mut self, idx: usize, item: T) -> Result<(), T> {
        self.try_insert(idx, item).map_err(|e| e.element())
    }

    #[inline]
    #[track_caller]
    fn remove(&mut self, idx: usize) -> T {
        self.remove(idx)
    }

    #[inline]
    fn retain(&mut self, f: impl FnMut(&mut T) -> bool) {
        self.retain(f);
    }

    #[inline]
    #[track_caller]
    fn splice(&mut self, range: impl RangeBounds<usize>, replacement: impl IntoIterator<Item = Self::Item>) {
        let mut i = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&x) => x,
            Bound::Excluded(&x) => x + 1,
        };
        self.drain(range);
        for x in replacement {
            self.insert(i, x);
            i += 1;
        }
    }
}

/// A helper trait for getting a range of items from a sorted slice.
pub trait SortedIndex<T, Q: ?Sized> {
    type Result: SliceIndex<[T], Output = [T]> + RangeBounds<usize>;
    fn find_range(self, slice: &[T], find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result;
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for RangeFull {
    type Result = RangeFull;
    fn find_range(self, _: &[T], _: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        self
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for Range<&Q> {
    type Result = Range<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, self.start);
        let (Ok(end) | Err(end)) = find(slice, self.end);
        Range { start, end }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for range::Range<&Q> {
    type Result = range::Range<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, self.start);
        let (Ok(end) | Err(end)) = find(slice, self.end);
        range::Range { start, end }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for RangeInclusive<&Q> {
    type Result = RangeInclusive<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, *self.start());
        let end = match find(slice, *self.end()) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        RangeInclusive::new(start, end)
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for range::RangeInclusive<&Q> {
    type Result = range::RangeInclusive<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, self.start);
        let end = match find(slice, self.end) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        range::RangeInclusive { start, end }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for RangeFrom<&Q> {
    type Result = RangeFrom<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, self.start);
        RangeFrom { start }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for range::RangeFrom<&Q> {
    type Result = range::RangeFrom<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(start) | Err(start)) = find(slice, self.start);
        range::RangeFrom { start }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for RangeTo<&Q> {
    type Result = RangeTo<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let (Ok(end) | Err(end)) = find(slice, self.end);
        RangeTo { end }
    }
}
impl<T, Q: ?Sized> SortedIndex<T, Q> for RangeToInclusive<&Q> {
    type Result = RangeToInclusive<usize>;
    #[inline]
    fn find_range(self, slice: &[T], mut find: impl FnMut(&[T], &Q) -> Result<usize, usize>) -> Self::Result {
        let end = match find(slice, self.end) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        RangeToInclusive { end }
    }
}

/// Gets the total number of steps in a range.
pub trait RangeLen {
    fn len(&self) -> usize;
}
impl RangeLen for Range<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}
impl RangeLen for range::Range<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}
impl RangeLen for RangeInclusive<usize> {
    #[inline]
    fn len(&self) -> usize {
        match self.end_bound() {
            Bound::Excluded(&x) => x - *self.start(),
            Bound::Included(&x) => x - *self.start() + 1,
            Bound::Unbounded => unreachable!(),
        }
    }
}
impl RangeLen for range::RangeInclusive<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start + 1
    }
}
impl RangeLen for RangeTo<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end
    }
}
impl RangeLen for RangeToInclusive<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end + 1
    }
}

/// Removes items from the current range which overlap with another.
///
/// The other range must start either before or at the current range, or it must end either at or
/// after the current range. i.e. `other.start <= self.start || self.end <= other.end`
pub trait SubtractRangeItemsFromEdge {
    fn subtract_range_items_from_edge(self, other: Range<usize>) -> Range<usize>;
}
impl SubtractRangeItemsFromEdge for Range<usize> {
    #[inline]
    fn subtract_range_items_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start <= self.start || self.end <= other.end);
        let (start, end) = if other.start <= self.start {
            (self.start.max(other.end).min(self.end), self.end)
        } else {
            (self.start, self.end.min(other.start))
        };
        Range { start, end }
    }
}
impl SubtractRangeItemsFromEdge for range::Range<usize> {
    #[inline]
    fn subtract_range_items_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start <= self.start || self.end <= other.end);
        let (start, end) = if other.start <= self.start {
            (self.start.max(other.end).min(self.end), self.end)
        } else {
            (self.start, self.end.min(other.start))
        };
        Range { start, end }
    }
}
impl SubtractRangeItemsFromEdge for RangeTo<usize> {
    #[inline]
    fn subtract_range_items_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start == 0 || self.end <= other.end);
        let (start, end) = if other.start == 0 {
            (other.end.min(self.end), self.end)
        } else {
            (0, self.end.min(other.start))
        };
        Range { start, end }
    }
}

/// Applies an exclusive upper limit to any explicit bounds in a range.
pub trait LimitExplicitRangeBounds {
    type Output: Clone
        + SliceIndex<[usize], Output = [usize]>
        + RangeBounds<usize>
        + LimitExplicitRangeBounds<Output = Self::Output>
        + IntoRangeWithStride;
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output;
}
impl LimitExplicitRangeBounds for usize {
    type Output = Range<usize>;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        if self < limit { self..self + 1 } else { limit..limit }
    }
}
impl LimitExplicitRangeBounds for RangeFull {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, _: usize) -> Self::Output {
        self
    }
}
impl LimitExplicitRangeBounds for Range<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitRangeBounds for range::Range<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitRangeBounds for RangeInclusive<usize> {
    type Output = Range<usize>;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Range {
            start: (*self.start()).min(limit),
            end: if *self.end() < limit {
                match self.end_bound() {
                    Bound::Included(&x) => x + 1,
                    Bound::Excluded(&x) => x,
                    Bound::Unbounded => limit,
                }
            } else {
                limit
            },
        }
    }
}
impl LimitExplicitRangeBounds for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        range::Range {
            start: self.start.min(limit),
            end: if self.end < limit { self.end + 1 } else { limit },
        }
    }
}
impl LimitExplicitRangeBounds for RangeTo<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Self {
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitRangeBounds for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        RangeTo {
            end: if self.end < limit { self.end + 1 } else { limit },
        }
    }
}
impl LimitExplicitRangeBounds for RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
        }
    }
}
impl LimitExplicitRangeBounds for range::RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_range_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
        }
    }
}

/// Adjusts a range/index to contain each item as though they were `n` steps apart (i.e. multiplies
/// the bounds by `n`).
pub trait IntoRangeWithStride {
    type Output: Clone
        + SliceIndex<[usize], Output = [usize]>
        + RangeBounds<usize>
        + LimitExplicitRangeBounds
        + IntoRangeWithStride<Output = Self::Output>;
    fn into_range_with_stride(self, stride: u32) -> Self::Output;
}
impl IntoRangeWithStride for usize {
    type Output = Range<usize>;
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        Range {
            start: self,
            end: self + stride as usize,
        }
    }
}
impl IntoRangeWithStride for RangeFull {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, _: u32) -> Self::Output {
        self
    }
}
impl IntoRangeWithStride for Range<usize> {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        Range {
            start: self.start * stride as usize,
            end: self.end * stride as usize,
        }
    }
}
impl IntoRangeWithStride for range::Range<usize> {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        range::Range {
            start: self.start * stride as usize,
            end: self.end * stride as usize,
        }
    }
}
impl IntoRangeWithStride for RangeInclusive<usize> {
    type Output = Range<usize>;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        Range {
            start: *self.start() * stride as usize,
            end: (*self.end() + 1) * stride as usize,
        }
    }
}
impl IntoRangeWithStride for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        range::Range {
            start: self.start * stride as usize,
            end: (self.end + 1) * stride as usize,
        }
    }
}
impl IntoRangeWithStride for RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        RangeFrom {
            start: self.start * stride as usize,
        }
    }
}
impl IntoRangeWithStride for range::RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        range::RangeFrom {
            start: self.start * stride as usize,
        }
    }
}
impl IntoRangeWithStride for RangeTo<usize> {
    type Output = Self;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        RangeTo {
            end: self.end * stride as usize,
        }
    }
}
impl IntoRangeWithStride for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn into_range_with_stride(self, stride: u32) -> Self::Output {
        RangeTo {
            end: (self.end + 1) * stride as usize,
        }
    }
}

/// Splits a range/index into a range before `n`, and the number of steps after `n`.
pub trait SplitRangeAt {
    type Output: Clone
        + SliceIndex<[usize], Output = [usize]>
        + RangeBounds<usize>
        + RangeLen
        + LimitExplicitRangeBounds
        + SubtractRangeItemsFromEdge
        + IntoRangeWithStride<Output: RangeLen + SubtractRangeItemsFromEdge>;
    fn split_range_at(self, idx: usize) -> (Self::Output, usize);
}
impl SplitRangeAt for usize {
    type Output = Range<usize>;
    #[inline]
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        if self < idx { (self..self + 1, 0) } else { (0..0, 1) }
    }
}
impl SplitRangeAt for Range<usize> {
    type Output = Range<usize>;
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range = Range {
            start: self.start.min(idx),
            end: self.end.min(idx),
        };
        let extra = self
            .end
            .wrapping_sub(self.start)
            .wrapping_sub(range.end.wrapping_sub(range.start));
        (range, if self.end > self.start { 0 } else { extra })
    }
}
impl SplitRangeAt for range::Range<usize> {
    type Output = range::Range<usize>;
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range = range::Range {
            start: self.start.min(idx),
            end: self.end.min(idx),
        };
        let extra = self
            .end
            .wrapping_sub(self.start)
            .wrapping_sub(range.end.wrapping_sub(range.start));
        (range, if self.end > self.start { 0 } else { extra })
    }
}
impl SplitRangeAt for RangeInclusive<usize> {
    type Output = Range<usize>;
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range = Range {
            start: (*self.start()).min(idx),
            end: (*self.end()).min(idx),
        };
        let extra = (*self.end())
            .wrapping_sub(*self.start())
            .wrapping_sub(range.end.wrapping_sub(range.start));
        // Can overflow if `count == 0` and the range is `0..=usize::MAX`.
        // Don't do that.
        (range, if self.is_empty() { 0 } else { extra + 1 })
    }
}
impl SplitRangeAt for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range = range::Range {
            start: self.start.min(idx),
            end: self.end.min(idx),
        };
        let extra = self
            .end
            .wrapping_sub(self.start)
            .wrapping_sub(range.end.wrapping_sub(range.start));
        // Can overflow if `count == 0` and the range is `0..=usize::MAX`.
        // Don't do that.
        (range, if self.start > self.end { 0 } else { extra + 1 })
    }
}
impl SplitRangeAt for RangeTo<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range: RangeTo<usize> = RangeTo { end: self.end.min(idx) };
        let extra = self.end.wrapping_sub(range.end);
        (range, extra)
    }
}
impl SplitRangeAt for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn split_range_at(self, idx: usize) -> (Self::Output, usize) {
        let range = RangeTo { end: self.end.min(idx) };
        // Can overflow if `count == 0` and the range is `..=usize::MAX`.
        // Don't do that.
        let extra = self.end.wrapping_sub(range.end) + 1;
        (range, extra)
    }
}

use core::ops::{Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::range;
use core::slice::SliceIndex;

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

use crate::bit_slice::Word;
use core::cmp::minmax;
use core::ops::{Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::range;
use core::slice::SliceIndex;

/// Gets the total number of steps in a range.
pub trait Len {
    /// Gets the total number of steps in a range.
    fn len(&self) -> usize;
}
impl Len for usize {
    #[inline]
    fn len(&self) -> usize {
        1
    }
}
impl Len for Range<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}
impl Len for range::Range<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
}
impl Len for RangeTo<usize> {
    #[inline]
    fn len(&self) -> usize {
        self.end
    }
}

/// Removes items from the current range which overlap with another.
///
/// The other range must either start before or at the current range, or it must end at or after the
/// current range. i.e. `other.start <= self.start || self.end <= other.end`
pub trait SubtractFromEdge {
    /// Removes items from the current range which overlap with another.
    fn subtract_from_edge(self, other: Range<usize>) -> Range<usize>;
}
impl SubtractFromEdge for usize {
    #[inline]
    fn subtract_from_edge(self, other: Range<usize>) -> Range<usize> {
        Range {
            start: self,
            end: self + usize::from(other.contains(&self)),
        }
    }
}
impl SubtractFromEdge for Range<usize> {
    #[inline]
    fn subtract_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start <= self.start || self.end <= other.end);
        let (start, end) = if other.start <= self.start {
            (self.start.max(other.end).min(self.end), self.end)
        } else {
            (self.start, self.end.min(other.start))
        };
        Range { start, end }
    }
}
impl SubtractFromEdge for range::Range<usize> {
    #[inline]
    fn subtract_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start <= self.start || self.end <= other.end);
        let (start, end) = if other.start <= self.start {
            (self.start.max(other.end).min(self.end), self.end)
        } else {
            (self.start, self.end.min(other.start))
        };
        Range { start, end }
    }
}
impl SubtractFromEdge for RangeTo<usize> {
    #[inline]
    fn subtract_from_edge(self, other: Range<usize>) -> Range<usize> {
        debug_assert!(other.start == 0 || self.end <= other.end);
        let (start, end) = if other.start == 0 {
            (other.end.min(self.end), self.end)
        } else {
            (0, self.end.min(other.start))
        };
        Range { start, end }
    }
}

/// Applies an exclusive upper limit to any explicit bounds in a range leaving implicit bounds
/// unchanged.
pub trait LimitExplicitBounds {
    type Output: Clone
        + SliceIndex<[Word], Output = [Word]>
        + RangeBounds<usize>
        + LimitExplicitBounds<Output = Self::Output>
        + WithStride;
    /// Applies an exclusive upper limit to any explicit bounds in a range leaving implicit bounds
    /// unchanged.
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output;
}
impl LimitExplicitBounds for usize {
    type Output = Range<usize>;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        if self < limit { self..self + 1 } else { limit..limit }
    }
}
impl LimitExplicitBounds for RangeFull {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, _: usize) -> Self::Output {
        self
    }
}
impl LimitExplicitBounds for Range<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitBounds for range::Range<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitBounds for RangeInclusive<usize> {
    type Output = Range<usize>;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Range {
            start: (*self.start()).min(limit),
            end: if *self.end() < limit {
                match self.end_bound() {
                    Bound::Included(&x) => x + 1,
                    Bound::Excluded(&x) => x,
                    Bound::Unbounded => unreachable!(),
                }
            } else {
                limit
            },
        }
    }
}
impl LimitExplicitBounds for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        range::Range {
            start: self.start.min(limit),
            end: if self.end < limit { self.end + 1 } else { limit },
        }
    }
}
impl LimitExplicitBounds for RangeTo<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Self {
            end: self.end.min(limit),
        }
    }
}
impl LimitExplicitBounds for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        RangeTo {
            end: if self.end < limit { self.end + 1 } else { limit },
        }
    }
}
impl LimitExplicitBounds for RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
        }
    }
}
impl LimitExplicitBounds for range::RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn limit_explicit_bounds(self, limit: usize) -> Self::Output {
        Self {
            start: self.start.min(limit),
        }
    }
}

/// Adjusts a range/index to contain each item as though they were `n` steps apart (i.e. multiplies
/// the bounds by `n`).
pub trait WithStride {
    type Output: Clone
        + SliceIndex<[Word], Output = [Word]>
        + RangeBounds<usize>
        + LimitExplicitBounds
        + WithStride<Output = Self::Output>;
    fn with_stride(self, stride: u32) -> Self::Output;
}
impl WithStride for usize {
    type Output = Range<usize>;
    fn with_stride(self, stride: u32) -> Self::Output {
        let start = self * stride as usize;
        Range {
            start,
            end: start + stride as usize,
        }
    }
}
impl WithStride for RangeFull {
    type Output = Self;
    #[inline]
    fn with_stride(self, _: u32) -> Self::Output {
        self
    }
}
impl WithStride for Range<usize> {
    type Output = Self;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        Range {
            start: self.start * stride as usize,
            end: self.end * stride as usize,
        }
    }
}
impl WithStride for range::Range<usize> {
    type Output = Self;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        range::Range {
            start: self.start * stride as usize,
            end: self.end * stride as usize,
        }
    }
}
impl WithStride for RangeInclusive<usize> {
    type Output = Range<usize>;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        Range {
            start: *self.start() * stride as usize,
            end: (*self.end() + 1) * stride as usize,
        }
    }
}
impl WithStride for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        range::Range {
            start: self.start * stride as usize,
            end: (self.end + 1) * stride as usize,
        }
    }
}
impl WithStride for RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        RangeFrom {
            start: self.start * stride as usize,
        }
    }
}
impl WithStride for range::RangeFrom<usize> {
    type Output = Self;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        range::RangeFrom {
            start: self.start * stride as usize,
        }
    }
}
impl WithStride for RangeTo<usize> {
    type Output = Self;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        RangeTo {
            end: self.end * stride as usize,
        }
    }
}
impl WithStride for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn with_stride(self, stride: u32) -> Self::Output {
        RangeTo {
            end: (self.end + 1) * stride as usize,
        }
    }
}

/// Splits a range/index into a range before `n`, and the number of steps after `n`.
pub trait SplitAt {
    type Output: Clone
        + SliceIndex<[Word], Output = [Word]>
        + RangeBounds<usize>
        + Len
        + LimitExplicitBounds
        + SubtractFromEdge
        + WithStride<Output: Len + SubtractFromEdge>;
    fn split_at(self, idx: usize) -> (Self::Output, usize);
}
impl SplitAt for usize {
    type Output = Range<usize>;
    #[inline]
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        if self < idx { (self..self + 1, 0) } else { (idx..idx, 1) }
    }
}
impl SplitAt for Range<usize> {
    type Output = Range<usize>;
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_start, post_start] = minmax(self.start, idx);
        let [pre_end, post_end] = minmax(self.end, idx);
        (
            Range {
                start: pre_start,
                end: pre_end,
            },
            post_end - post_start,
        )
    }
}
impl SplitAt for range::Range<usize> {
    type Output = range::Range<usize>;
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_start, post_start] = minmax(self.start, idx);
        let [pre_end, post_end] = minmax(self.end, idx);
        (
            range::Range {
                start: pre_start,
                end: pre_end,
            },
            post_end - post_start,
        )
    }
}
impl SplitAt for RangeInclusive<usize> {
    type Output = Range<usize>;
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_start, post_start] = minmax(*self.start(), idx);
        let [pre_end, post_end] = minmax(
            match self.end_bound() {
                Bound::Unbounded => 0,
                Bound::Excluded(&x) => x,
                // will result in invalid or empty ranges on overflow.
                Bound::Included(&x) => x + 1,
            },
            idx,
        );
        (
            Range {
                start: pre_start,
                end: pre_end,
            },
            post_end - post_start,
        )
    }
}
impl SplitAt for range::RangeInclusive<usize> {
    type Output = range::Range<usize>;
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_start, post_start] = minmax(self.start, idx);
        let [pre_end, post_end] = minmax(self.end + 1, idx);
        (
            range::Range {
                start: pre_start,
                end: pre_end,
            },
            post_end - post_start,
        )
    }
}
impl SplitAt for RangeTo<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_end, post_end] = minmax(self.end, idx);
        (RangeTo { end: pre_end }, post_end - idx)
    }
}
impl SplitAt for RangeToInclusive<usize> {
    type Output = RangeTo<usize>;
    #[inline]
    fn split_at(self, idx: usize) -> (Self::Output, usize) {
        let [pre_end, post_end] = minmax(self.end + 1, idx);
        (RangeTo { end: pre_end }, post_end - idx)
    }
}

#[test]
fn len() {
    assert_eq!(Len::len(&0), 1);
    assert_eq!(Len::len(&Range { start: 0, end: 0 }), 0);
    assert_eq!(Len::len(&range::Range { start: 0, end: 0 }), 0);
    assert_eq!(Len::len(&RangeTo { end: 0 }), 0);

    assert_eq!(Len::len(&Range { start: 0, end: 1 }), 1);
    assert_eq!(Len::len(&range::Range { start: 0, end: 1 }), 1);
    assert_eq!(Len::len(&RangeTo { end: 1 }), 1);

    assert_eq!(
        Len::len(&Range {
            start: 0,
            end: usize::MAX
        }),
        usize::MAX
    );
    assert_eq!(
        Len::len(&range::Range {
            start: 0,
            end: usize::MAX
        }),
        usize::MAX
    );
    assert_eq!(Len::len(&RangeTo { end: usize::MAX }), usize::MAX);
}

#[test]
#[expect(clippy::too_many_lines)]
fn subtract_from_edge() {
    assert_eq!(
        Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 0, end: 1 }),
        // `0..0`` would also be acceptable
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 3 }),
        Range { start: 3, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 3, end: 3 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 3, end: 4 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 2, end: 3 }),
        Range { start: 1, end: 2 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 2, end: 4 }),
        Range { start: 1, end: 2 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 4 }),
        // `1..1` would alsop be acceptable
        Range { start: 3, end: 3 },
    );
    assert_eq!(
        Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 4 }),
        // `1..1` would alsop be acceptable
        Range { start: 3, end: 3 },
    );

    assert_eq!(
        range::Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 0 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 0, end: 1 }),
        // `0..0`` would also be acceptable
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        range::Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        range::Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        range::Range { start: 1, end: 1 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 3 }),
        Range { start: 3, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 2, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 3, end: 3 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 3, end: 4 }),
        Range { start: 1, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 2, end: 3 }),
        Range { start: 1, end: 2 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 2, end: 4 }),
        Range { start: 1, end: 2 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 1, end: 4 }),
        // `1..1` would alsop be acceptable
        Range { start: 3, end: 3 },
    );
    assert_eq!(
        range::Range { start: 1, end: 3 }.subtract_from_edge(Range { start: 0, end: 4 }),
        // `1..1` would alsop be acceptable
        Range { start: 3, end: 3 },
    );

    // RangeTo
    assert_eq!(
        RangeTo { end: 0 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeTo { end: 0 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeTo { end: 0 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeTo { end: 1 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        RangeTo { end: 1 }.subtract_from_edge(Range { start: 0, end: 1 }),
        // `0..0`` would also be acceptable
        Range { start: 1, end: 1 },
    );
    assert_eq!(
        RangeTo { end: 1 }.subtract_from_edge(Range { start: 1, end: 1 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        RangeTo { end: 1 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 0, end: 0 }),
        Range { start: 0, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 0, end: 1 }),
        Range { start: 1, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 0, end: 2 }),
        Range { start: 2, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 0, end: 3 }),
        Range { start: 2, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 2, end: 2 }),
        Range { start: 0, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 2, end: 3 }),
        Range { start: 0, end: 2 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 1, end: 2 }),
        Range { start: 0, end: 1 },
    );
    assert_eq!(
        RangeTo { end: 2 }.subtract_from_edge(Range { start: 1, end: 3 }),
        Range { start: 0, end: 1 },
    );
}

#[test]
fn limit_explicit_bounds() {
    assert_eq!(0.limit_explicit_bounds(0), Range { start: 0, end: 0 });
    assert_eq!(0.limit_explicit_bounds(1), Range { start: 0, end: 1 });
    assert_eq!(1.limit_explicit_bounds(1), Range { start: 1, end: 1 });
    assert_eq!(5.limit_explicit_bounds(2), Range { start: 2, end: 2 });

    assert_eq!(
        Range { start: 0, end: 0 }.limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 0, end: 1 }.limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 2, end: 4 }.limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        Range { start: 1, end: 20 }.limit_explicit_bounds(5),
        Range { start: 1, end: 5 },
    );

    assert_eq!(
        range::Range { start: 0, end: 0 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 2, end: 4 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 1, end: 20 }.limit_explicit_bounds(5),
        range::Range { start: 1, end: 5 },
    );

    assert_eq!(
        RangeInclusive::new(0, 0).limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeInclusive::new(0, 1).limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeInclusive::new(2, 4).limit_explicit_bounds(0),
        Range { start: 0, end: 0 },
    );
    assert_eq!(
        RangeInclusive::new(1, 20).limit_explicit_bounds(5),
        Range { start: 1, end: 5 },
    );

    assert_eq!(
        range::RangeInclusive { start: 0, end: 0 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 1 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::RangeInclusive { start: 2, end: 4 }.limit_explicit_bounds(0),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::RangeInclusive { start: 1, end: 20 }.limit_explicit_bounds(5),
        range::Range { start: 1, end: 5 },
    );

    assert_eq!(RangeTo { end: 0 }.limit_explicit_bounds(0), RangeTo { end: 0 },);
    assert_eq!(RangeTo { end: 1 }.limit_explicit_bounds(0), RangeTo { end: 0 },);
    assert_eq!(RangeTo { end: 20 }.limit_explicit_bounds(5), RangeTo { end: 5 },);

    assert_eq!(RangeToInclusive { end: 0 }.limit_explicit_bounds(0), RangeTo { end: 0 },);
    assert_eq!(RangeToInclusive { end: 1 }.limit_explicit_bounds(0), RangeTo { end: 0 },);
    assert_eq!(
        RangeToInclusive { end: 20 }.limit_explicit_bounds(5),
        RangeTo { end: 5 },
    );

    assert_eq!(RangeFrom { start: 0 }.limit_explicit_bounds(0), RangeFrom { start: 0 },);
    assert_eq!(RangeFrom { start: 1 }.limit_explicit_bounds(0), RangeFrom { start: 0 },);
    assert_eq!(RangeFrom { start: 20 }.limit_explicit_bounds(5), RangeFrom { start: 5 },);

    assert_eq!(
        range::RangeFrom { start: 0 }.limit_explicit_bounds(0),
        range::RangeFrom { start: 0 },
    );
    assert_eq!(
        range::RangeFrom { start: 1 }.limit_explicit_bounds(0),
        range::RangeFrom { start: 0 },
    );
    assert_eq!(
        range::RangeFrom { start: 20 }.limit_explicit_bounds(5),
        range::RangeFrom { start: 5 },
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn with_stride() {
    assert_eq!(0.with_stride(1), Range { start: 0, end: 1 });
    assert_eq!(0.with_stride(2), Range { start: 0, end: 2 });
    assert_eq!(1.with_stride(1), Range { start: 1, end: 2 });
    assert_eq!(1.with_stride(2), Range { start: 2, end: 4 });
    assert_eq!(2.with_stride(4), Range { start: 8, end: 12 });

    assert_eq!(Range { start: 0, end: 0 }.with_stride(1), Range { start: 0, end: 0 },);
    assert_eq!(Range { start: 0, end: 1 }.with_stride(1), Range { start: 0, end: 1 },);
    assert_eq!(Range { start: 2, end: 6 }.with_stride(1), Range { start: 2, end: 6 },);
    assert_eq!(Range { start: 0, end: 0 }.with_stride(2), Range { start: 0, end: 0 },);
    assert_eq!(Range { start: 0, end: 2 }.with_stride(2), Range { start: 0, end: 4 },);
    assert_eq!(Range { start: 4, end: 10 }.with_stride(5), Range { start: 20, end: 50 },);

    assert_eq!(
        range::Range { start: 0, end: 0 }.with_stride(1),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.with_stride(1),
        range::Range { start: 0, end: 1 },
    );
    assert_eq!(
        range::Range { start: 2, end: 6 }.with_stride(1),
        range::Range { start: 2, end: 6 },
    );
    assert_eq!(
        range::Range { start: 0, end: 0 }.with_stride(2),
        range::Range { start: 0, end: 0 },
    );
    assert_eq!(
        range::Range { start: 0, end: 2 }.with_stride(2),
        range::Range { start: 0, end: 4 },
    );
    assert_eq!(
        range::Range { start: 4, end: 10 }.with_stride(5),
        range::Range { start: 20, end: 50 },
    );

    assert_eq!(RangeInclusive::new(0, 0).with_stride(1), Range { start: 0, end: 1 },);
    assert_eq!(RangeInclusive::new(0, 1).with_stride(1), Range { start: 0, end: 2 },);
    assert_eq!(RangeInclusive::new(2, 6).with_stride(1), Range { start: 2, end: 7 },);
    assert_eq!(RangeInclusive::new(0, 0).with_stride(2), Range { start: 0, end: 2 },);
    assert_eq!(RangeInclusive::new(0, 2).with_stride(2), Range { start: 0, end: 6 },);
    assert_eq!(RangeInclusive::new(4, 10).with_stride(5), Range { start: 20, end: 55 },);

    assert_eq!(
        range::RangeInclusive { start: 0, end: 0 }.with_stride(1),
        range::Range { start: 0, end: 1 },
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 1 }.with_stride(1),
        range::Range { start: 0, end: 2 },
    );
    assert_eq!(
        range::RangeInclusive { start: 2, end: 6 }.with_stride(1),
        range::Range { start: 2, end: 7 },
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 0 }.with_stride(2),
        range::Range { start: 0, end: 2 },
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 2 }.with_stride(2),
        range::Range { start: 0, end: 6 },
    );
    assert_eq!(
        range::RangeInclusive { start: 4, end: 10 }.with_stride(5),
        range::Range { start: 20, end: 55 },
    );

    assert_eq!(RangeTo { end: 0 }.with_stride(1), RangeTo { end: 0 },);
    assert_eq!(RangeTo { end: 1 }.with_stride(1), RangeTo { end: 1 },);
    assert_eq!(RangeTo { end: 6 }.with_stride(1), RangeTo { end: 6 },);
    assert_eq!(RangeTo { end: 0 }.with_stride(2), RangeTo { end: 0 },);
    assert_eq!(RangeTo { end: 2 }.with_stride(2), RangeTo { end: 4 },);
    assert_eq!(RangeTo { end: 10 }.with_stride(5), RangeTo { end: 50 },);

    assert_eq!(RangeToInclusive { end: 0 }.with_stride(1), RangeTo { end: 1 },);
    assert_eq!(RangeToInclusive { end: 1 }.with_stride(1), RangeTo { end: 2 },);
    assert_eq!(RangeToInclusive { end: 6 }.with_stride(1), RangeTo { end: 7 },);
    assert_eq!(RangeToInclusive { end: 0 }.with_stride(2), RangeTo { end: 2 },);
    assert_eq!(RangeToInclusive { end: 2 }.with_stride(2), RangeTo { end: 6 },);
    assert_eq!(RangeToInclusive { end: 10 }.with_stride(5), RangeTo { end: 55 },);

    assert_eq!(RangeFrom { start: 0 }.with_stride(1), RangeFrom { start: 0 },);
    assert_eq!(RangeFrom { start: 1 }.with_stride(1), RangeFrom { start: 1 },);
    assert_eq!(RangeFrom { start: 6 }.with_stride(1), RangeFrom { start: 6 },);
    assert_eq!(RangeFrom { start: 0 }.with_stride(2), RangeFrom { start: 0 },);
    assert_eq!(RangeFrom { start: 2 }.with_stride(2), RangeFrom { start: 4 },);
    assert_eq!(RangeFrom { start: 10 }.with_stride(5), RangeFrom { start: 50 },);

    assert_eq!(
        range::RangeFrom { start: 0 }.with_stride(1),
        range::RangeFrom { start: 0 },
    );
    assert_eq!(
        range::RangeFrom { start: 1 }.with_stride(1),
        range::RangeFrom { start: 1 },
    );
    assert_eq!(
        range::RangeFrom { start: 6 }.with_stride(1),
        range::RangeFrom { start: 6 },
    );
    assert_eq!(
        range::RangeFrom { start: 0 }.with_stride(2),
        range::RangeFrom { start: 0 },
    );
    assert_eq!(
        range::RangeFrom { start: 2 }.with_stride(2),
        range::RangeFrom { start: 4 },
    );
    assert_eq!(
        range::RangeFrom { start: 10 }.with_stride(5),
        range::RangeFrom { start: 50 },
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn split_at() {
    assert_eq!(0.split_at(0), (Range { start: 0, end: 0 }, 1));
    assert_eq!(0.split_at(1), (Range { start: 0, end: 1 }, 0));
    assert_eq!(1.split_at(0), (Range { start: 0, end: 0 }, 1));
    assert_eq!(1.split_at(1), (Range { start: 1, end: 1 }, 1));
    assert_eq!(5.split_at(20), (Range { start: 5, end: 6 }, 0));

    assert_eq!(Range { start: 0, end: 0 }.split_at(0), (Range { start: 0, end: 0 }, 0),);
    assert_eq!(Range { start: 0, end: 1 }.split_at(0), (Range { start: 0, end: 0 }, 1),);
    assert_eq!(Range { start: 0, end: 0 }.split_at(1), (Range { start: 0, end: 0 }, 0),);
    assert_eq!(Range { start: 0, end: 5 }.split_at(1), (Range { start: 0, end: 1 }, 4),);
    assert_eq!(Range { start: 1, end: 1 }.split_at(0), (Range { start: 0, end: 0 }, 0),);
    assert_eq!(Range { start: 1, end: 2 }.split_at(0), (Range { start: 0, end: 0 }, 1),);
    assert_eq!(Range { start: 1, end: 1 }.split_at(1), (Range { start: 1, end: 1 }, 0),);
    assert_eq!(Range { start: 1, end: 5 }.split_at(2), (Range { start: 1, end: 2 }, 3),);
    assert_eq!(
        Range { start: 20, end: 200 }.split_at(55),
        (Range { start: 20, end: 55 }, 145),
    );

    assert_eq!(
        range::Range { start: 0, end: 0 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 0),
    );
    assert_eq!(
        range::Range { start: 0, end: 1 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 1),
    );
    assert_eq!(
        range::Range { start: 0, end: 0 }.split_at(1),
        (range::Range { start: 0, end: 0 }, 0),
    );
    assert_eq!(
        range::Range { start: 0, end: 5 }.split_at(1),
        (range::Range { start: 0, end: 1 }, 4),
    );
    assert_eq!(
        range::Range { start: 1, end: 1 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 0),
    );
    assert_eq!(
        range::Range { start: 1, end: 2 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 1),
    );
    assert_eq!(
        range::Range { start: 1, end: 1 }.split_at(1),
        (range::Range { start: 1, end: 1 }, 0),
    );
    assert_eq!(
        range::Range { start: 1, end: 5 }.split_at(2),
        (range::Range { start: 1, end: 2 }, 3),
    );
    assert_eq!(
        range::Range { start: 20, end: 200 }.split_at(55),
        (range::Range { start: 20, end: 55 }, 145),
    );

    assert_eq!(RangeInclusive::new(0, 0).split_at(0), (Range { start: 0, end: 0 }, 1),);
    assert_eq!(RangeInclusive::new(0, 1).split_at(0), (Range { start: 0, end: 0 }, 2),);
    assert_eq!(RangeInclusive::new(0, 0).split_at(1), (Range { start: 0, end: 1 }, 0),);
    assert_eq!(RangeInclusive::new(0, 5).split_at(1), (Range { start: 0, end: 1 }, 5),);
    assert_eq!(RangeInclusive::new(1, 1).split_at(0), (Range { start: 0, end: 0 }, 1),);
    assert_eq!(RangeInclusive::new(1, 2).split_at(0), (Range { start: 0, end: 0 }, 2),);
    assert_eq!(RangeInclusive::new(1, 1).split_at(1), (Range { start: 1, end: 1 }, 1),);
    assert_eq!(RangeInclusive::new(1, 5).split_at(2), (Range { start: 1, end: 2 }, 4),);
    assert_eq!(
        RangeInclusive::new(20, 200).split_at(55),
        (Range { start: 20, end: 55 }, 146),
    );

    assert_eq!(
        range::RangeInclusive { start: 0, end: 0 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 1),
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 1 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 2),
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 0 }.split_at(1),
        (range::Range { start: 0, end: 1 }, 0),
    );
    assert_eq!(
        range::RangeInclusive { start: 0, end: 5 }.split_at(1),
        (range::Range { start: 0, end: 1 }, 5),
    );
    assert_eq!(
        range::RangeInclusive { start: 1, end: 1 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 1),
    );
    assert_eq!(
        range::RangeInclusive { start: 1, end: 2 }.split_at(0),
        (range::Range { start: 0, end: 0 }, 2),
    );
    assert_eq!(
        range::RangeInclusive { start: 1, end: 1 }.split_at(1),
        (range::Range { start: 1, end: 1 }, 1),
    );
    assert_eq!(
        range::RangeInclusive { start: 1, end: 5 }.split_at(2),
        (range::Range { start: 1, end: 2 }, 4),
    );
    assert_eq!(
        range::RangeInclusive { start: 20, end: 200 }.split_at(55),
        (range::Range { start: 20, end: 55 }, 146),
    );
}

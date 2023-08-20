use crate::BitSlice;
use crate::bit_slice::{Word, final_mask_for_size, word_count_from_bits};
use crate::traits::{
    IntoRangeWithStride, LimitExplicitRangeBounds, RangeLen, SplitRangeAt, SubtractRangeItemsFromEdge,
};
use core::iter;
use core::marker::PhantomData;
use rustc_arena::DroplessArena;
use rustc_index::{Idx, IntoSliceIdx};

/// A reference to a two-dimensional bit set.
///
/// This is represented as a dense array of words stored in row major order with each row aligned to
/// the start of a word.
pub struct BitSlice2d<'a, R, C> {
    words: &'a mut [Word],
    rows: u32,
    columns: u32,
    row_stride: u32,
    phantom: PhantomData<(R, C)>,
}
impl<'a, R, C> BitSlice2d<'a, R, C> {
    /// Treats `words` as a two-dimensional bit set.
    ///
    /// The length of the given slice must match the number of words required to store a bit set
    /// with the given dimensions.
    #[inline]
    pub fn from_mut_words(words: &'a mut [Word], rows: u32, columns: u32) -> Self {
        let row_stride = word_count_from_bits(columns as usize);
        debug_assert_eq!(Some(words.len()), row_stride.checked_mul(rows as usize));
        Self {
            words,
            rows,
            columns,
            row_stride: row_stride as u32,
            phantom: PhantomData,
        }
    }

    /// Allocates a new zero-initialized, two-dimensional bit set of the given size.
    #[inline]
    pub fn empty_arena(arena: &'a DroplessArena, rows: u32, columns: u32) -> Self {
        let row_stride = word_count_from_bits(columns as usize);
        Self {
            words: arena.alloc_from_iter(iter::repeat_n(0usize, row_stride.checked_mul(rows as usize).unwrap())),
            rows,
            columns,
            row_stride: row_stride as u32,
            phantom: PhantomData,
        }
    }

    /// Gets the number of rows in the bit set.
    #[inline]
    pub const fn row_len(&self) -> u32 {
        self.rows
    }

    /// Gets the number of columns in the bit set.
    #[inline]
    pub const fn column_len(&self) -> u32 {
        self.columns
    }

    /// Gets a reference to the words backing this bit set.
    #[inline]
    pub const fn words(&self) -> &[Word] {
        self.words
    }

    /// Gets a mutable reference to the words backing this bit set.
    #[inline]
    pub fn words_mut(&mut self) -> &mut [Word] {
        self.words
    }

    #[inline]
    #[track_caller]
    pub fn iter_rows(
        &self,
        range: impl IntoSliceIdx<R, [usize], Output: IntoRangeWithStride>,
    ) -> impl ExactSizeIterator<Item = &BitSlice<C>> + Clone {
        self.words[range.into_slice_idx().into_range_with_stride(self.row_stride)]
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
    }

    #[inline]
    pub fn iter_mut_rows(
        &mut self,
        range: impl IntoSliceIdx<R, [usize], Output: IntoRangeWithStride>,
    ) -> impl ExactSizeIterator<Item = &mut BitSlice<C>> {
        self.words[range.into_slice_idx().into_range_with_stride(self.row_stride)]
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&x| x == 0)
    }

    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|&x| x.count_ones() as usize).sum()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    #[inline]
    pub fn fill(&mut self) {
        self.words.fill(!0);
        let mask = final_mask_for_size(self.columns as usize);
        for row in self.iter_mut_rows(..) {
            row.mask_final_word(mask);
        }
    }

    pub fn union(&mut self, other: &BitSlice2d<'_, R, C>) -> bool {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.columns, other.columns);
        self.words.iter_mut().zip(&*other.words).fold(false, |res, (dst, src)| {
            let prev = *dst;
            *dst |= *src;
            res || prev != *dst
        })
    }
}
impl<'a, R: Idx, C: Idx> BitSlice2d<'a, R, C> {
    #[inline]
    pub fn enumerate_rows(&self) -> impl ExactSizeIterator<Item = (R, &BitSlice<C>)> + Clone {
        self.words
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    #[inline]
    pub fn enumerate_rows_mut(&mut self) -> impl ExactSizeIterator<Item = (R, &mut BitSlice<C>)> {
        self.words
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    #[inline]
    #[track_caller]
    pub fn row(&self, row: R) -> &BitSlice<C> {
        assert!(row.index() < self.rows as usize);
        let start = self.row_stride as usize * row.index();
        BitSlice::from_words(&self.words[start..start + self.row_stride as usize])
    }

    #[inline]
    #[track_caller]
    pub fn row_mut(&mut self, row: R) -> &mut BitSlice<C> {
        assert!(row.index() < self.rows as usize);
        let start = self.row_stride as usize * row.index();
        BitSlice::from_words_mut(&mut self.words[start..start + self.row_stride as usize])
    }

    #[inline]
    #[track_caller]
    pub fn copy_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: IntoRangeWithStride>, dst: R) {
        let src = src.into_slice_idx().into_range_with_stride(self.row_stride);
        self.words.copy_within(src, dst.index());
    }

    #[inline]
    #[track_caller]
    pub fn move_rows(
        &mut self,
        src: impl IntoSliceIdx<R, [usize], Output: IntoRangeWithStride<Output: RangeLen + SubtractRangeItemsFromEdge>>,
        dst: R,
    ) {
        let src = src.into_slice_idx().into_range_with_stride(self.row_stride);
        let dst_start = dst.index() * self.row_stride as usize;
        self.words.copy_within(src.clone(), dst_start);
        let src_len = src.len();
        self.words[src.subtract_range_items_from_edge(dst_start..dst_start + src_len)].fill(0);
    }

    #[inline]
    #[track_caller]
    pub fn clear_rows(&mut self, rows: impl IntoSliceIdx<R, [usize], Output: IntoRangeWithStride>) {
        let words = &mut self.words[rows.into_slice_idx().into_range_with_stride(self.row_stride)];
        words.fill(0);
    }
}

impl<R, C> PartialEq for BitSlice2d<'_, R, C> {
    fn eq(&self, other: &Self) -> bool {
        self.columns == other.columns && self.rows == other.rows && self.words == other.words
    }
}
impl<R, C> Eq for BitSlice2d<'_, R, C> {}

pub struct GrowableBitSet2d<R, C> {
    words: Vec<Word>,
    rows: u32,
    columns: u32,
    row_stride: u32,
    phantom: PhantomData<(R, C)>,
}
impl<R, C> GrowableBitSet2d<R, C> {
    #[inline]
    pub const fn new(columns: u32) -> Self {
        Self {
            words: Vec::new(),
            rows: 0,
            columns,
            row_stride: word_count_from_bits(columns as usize) as u32,
            phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn row_len(&self) -> u32 {
        self.rows
    }

    #[inline]
    pub const fn column_len(&self) -> u32 {
        self.columns
    }

    #[inline]
    pub fn words(&self) -> &[Word] {
        self.words.as_slice()
    }

    #[inline]
    pub fn words_mut(&mut self) -> &mut [Word] {
        self.words.as_mut_slice()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&x| x == 0)
    }

    #[inline]
    pub fn iter_rows(
        &self,
        range: impl IntoSliceIdx<R, [usize], Output: LimitExplicitRangeBounds>,
    ) -> impl ExactSizeIterator<Item = &BitSlice<C>> + Clone {
        self.words[range
            .into_slice_idx()
            .limit_explicit_range_bounds(self.rows as usize)
            .into_range_with_stride(self.row_stride)]
        .chunks_exact(self.row_stride as usize)
        .map(|words| BitSlice::from_words(words))
    }

    #[inline]
    pub fn iter_mut_rows(
        &mut self,
        range: impl IntoSliceIdx<R, [usize], Output: LimitExplicitRangeBounds>,
    ) -> impl ExactSizeIterator<Item = &mut BitSlice<C>> {
        self.words[range
            .into_slice_idx()
            .limit_explicit_range_bounds(self.rows as usize)
            .into_range_with_stride(self.row_stride)]
        .chunks_exact_mut(self.row_stride as usize)
        .map(|words| BitSlice::from_words_mut(words))
    }

    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|&x| x.count_ones() as usize).sum()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.words.clear();
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> BitSlice2d<'_, R, C> {
        BitSlice2d {
            words: self.words.as_mut_slice(),
            rows: self.rows,
            columns: self.columns,
            row_stride: self.row_stride,
            phantom: PhantomData,
        }
    }

    pub fn union(&mut self, other: &Self) -> bool {
        assert_eq!(self.columns, other.columns);
        if self.rows < other.rows {
            self.words.resize(other.row_stride as usize * other.rows as usize, 0);
            self.rows = other.rows;
        }
        self.words.iter_mut().zip(&*other.words).fold(false, |res, (dst, src)| {
            let prev = *dst;
            *dst |= *src;
            res || prev != *dst
        })
    }
}
impl<R: Idx, C: Idx> GrowableBitSet2d<R, C> {
    #[inline]
    pub fn enumerate_rows(&self) -> impl ExactSizeIterator<Item = (R, &BitSlice<C>)> + Clone {
        self.words
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    #[inline]
    pub fn enumerate_mut_rows(&mut self) -> impl ExactSizeIterator<Item = (R, &mut BitSlice<C>)> {
        self.words
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    #[inline]
    pub fn opt_row(&self, row: R) -> Option<&BitSlice<C>> {
        let start = self.row_stride as usize * row.index();
        self.words
            .get(start..start + self.row_stride as usize)
            .map(BitSlice::from_words)
    }

    #[inline]
    pub fn ensure_row(&mut self, row: R) -> &mut BitSlice<C> {
        let start = self.row_stride as usize * row.index();
        let end = start + self.row_stride as usize;
        BitSlice::from_words_mut(match self.words.get_mut(start..end) {
            Some(_) => &mut self.words[start..end],
            None => {
                self.words.resize(end, 0);
                self.rows = row.index() as u32 + 1;
                &mut self.words[start..end]
            },
        })
    }

    #[inline]
    pub fn clear_rows(&mut self, rows: impl IntoSliceIdx<R, [usize], Output: LimitExplicitRangeBounds>) {
        self.words[rows
            .into_slice_idx()
            .limit_explicit_range_bounds(self.rows as usize)
            .into_range_with_stride(self.row_stride)]
        .fill(0);
    }

    pub fn copy_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: SplitRangeAt>, dst: R) {
        let (src_range, src_extra) = src.into_slice_idx().split_range_at(self.rows as usize);
        let dst_start = dst.index() * self.row_stride as usize;
        let dst_row_end = dst.index() + src_range.len();
        let src_range = src_range.into_range_with_stride(self.row_stride);
        let dst_copy_end = dst_start + src_range.len();
        if self.rows < dst_row_end as u32 {
            self.words.resize(dst_copy_end, 0);
            self.rows = dst_row_end as u32;
        }
        self.words.copy_within(src_range, dst_start);
        let dst_end = dst_copy_end + self.words.len().min(src_extra * self.row_stride as usize);
        self.words[dst_copy_end..dst_end].fill(0);
    }

    pub fn move_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: SplitRangeAt>, dst: R) {
        let (src_range, src_extra) = src.into_slice_idx().split_range_at(self.rows as usize);
        let dst_start = dst.index() * self.row_stride as usize;
        let dst_row_end = dst.index() + src_range.len();
        let src_range = src_range.into_range_with_stride(self.row_stride);
        let dst_copy_end = dst_start + src_range.len();
        if self.rows < dst_row_end as u32 {
            self.words.resize(dst_copy_end, 0);
            self.rows = dst_row_end as u32;
        }
        self.words.copy_within(src_range.clone(), dst_start);
        let dst_end = self
            .words
            .len()
            .min(dst_copy_end + src_extra * self.row_stride as usize);
        self.words[dst_copy_end..dst_end].fill(0);
        self.words[src_range.subtract_range_items_from_edge(dst_start..dst_end)].fill(0);
    }
}

impl<R, C> PartialEq for GrowableBitSet2d<R, C> {
    fn eq(&self, other: &Self) -> bool {
        if self.columns != other.columns {
            return false;
        }
        let (lhs, rhs, extra) = match self.words.split_at_checked(other.words.len()) {
            Some((lhs, extra)) => (lhs, other.words.as_slice(), extra),
            None => {
                let (rhs, extra) = other.words.split_at(self.words.len());
                (self.words.as_slice(), rhs, extra)
            },
        };
        lhs == rhs && extra.iter().all(|&x| x == 0)
    }
}
impl<R, C> Eq for GrowableBitSet2d<R, C> {}

impl<R, C> Clone for GrowableBitSet2d<R, C> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            rows: self.rows,
            columns: self.columns,
            row_stride: self.row_stride,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.words.clone_from(&source.words);
        self.rows = source.rows;
        self.columns = source.columns;
        self.row_stride = source.row_stride;
    }
}

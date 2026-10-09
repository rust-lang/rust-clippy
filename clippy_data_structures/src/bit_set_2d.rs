use crate::bit_slice::{BitSlice, Word, final_mask_for_size, word_count_from_bits};
use crate::range::{self, Len as _, LimitExplicitBounds, SplitAt as _, SubtractFromEdge, WithStride};
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
    /// Interprets `words` as a two-dimensional bit set of the given size.
    ///
    /// The length of the given slice must match the number of words required to store a bit set
    /// with the given dimensions.
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
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

    /// Allocates a new empty two-dimensional bit set of the given size.
    ///
    /// # Panics
    /// Panics if `rows * columns` overflows a usize.
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn empty_arena(arena: &'a DroplessArena, rows: u32, columns: u32) -> Self {
        let row_stride = word_count_from_bits(columns as usize);
        Self {
            words: arena.alloc_from_iter(iter::repeat_n(0, row_stride.checked_mul(rows as usize).unwrap())),
            rows,
            columns,
            row_stride: row_stride as u32,
            phantom: PhantomData,
        }
    }

    /// Gets the number of rows.
    #[inline]
    #[must_use]
    pub const fn row_len(&self) -> u32 {
        self.rows
    }

    /// Gets the number of columns.
    #[inline]
    #[must_use]
    pub const fn column_len(&self) -> u32 {
        self.columns
    }

    /// Get the backing slice of words.
    #[inline]
    #[must_use]
    pub const fn words(&self) -> &[Word] {
        self.words
    }

    /// Get the backing slice of words.
    #[inline]
    #[must_use]
    pub fn words_mut(&mut self) -> &mut [Word] {
        self.words
    }

    /// Creates an iterator over the given rows.
    ///
    /// # Panics
    /// Panics if the range exceeds the number of rows.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn iter_rows(
        &self,
        range: impl IntoSliceIdx<R, [usize], Output: WithStride>,
    ) -> impl ExactSizeIterator<Item = &BitSlice<C>> + Clone {
        self.words[range.into_slice_idx().with_stride(self.row_stride)]
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
    }

    /// Creates an iterator over the given rows.
    ///
    /// # Panics
    /// Panics if the range exceeds the number of rows.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn iter_mut_rows(
        &mut self,
        range: impl IntoSliceIdx<R, [usize], Output: WithStride>,
    ) -> impl ExactSizeIterator<Item = &mut BitSlice<C>> {
        self.words[range.into_slice_idx().with_stride(self.row_stride)]
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
    }

    /// Checks if the set is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&x| x == 0)
    }

    /// Counts the number of elements in the set.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.words.iter().map(|&x| x.count_ones() as usize).sum()
    }

    /// Remove all elements from the set.
    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Inserts all elements into the set.
    #[inline]
    pub fn insert_all(&mut self) {
        self.words.fill(!0);
        let mask = final_mask_for_size(self.columns as usize);
        for row in self.iter_mut_rows(..) {
            row.mask_final_word(mask);
        }
    }

    /// Performs a union of two sets storing the result in `self`. Returns `true` if `self` has
    /// changed.
    ///
    /// # Panics
    /// Panics if the sets contain a different number of either rows or columns.
    pub fn union(&mut self, other: &BitSlice2d<'_, R, C>) -> bool {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.columns, other.columns);
        self.words.iter_mut().zip(&*other.words).fold(false, |res, (dst, src)| {
            let prev = *dst;
            *dst |= *src;
            res || prev != *dst
        })
    }

    /// Performs a subtraction of other from `self` storing the result in `self`. Returns `true` if
    /// `self` has changed.
    ///
    /// # Panics
    /// Panics if the sets contain a different number of either rows or columns.
    pub fn subtract(&mut self, other: &BitSlice2d<'_, R, C>) -> bool {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.columns, other.columns);
        self.words.iter_mut().zip(&*other.words).fold(false, |res, (dst, src)| {
            let prev = *dst;
            *dst &= !*src;
            res || prev != *dst
        })
    }

    /// Performs an intersection of two sets storing the result in `self`. Returns `true` if `self`
    /// has changed.
    ///
    /// # Panics
    /// Panics if the sets contain a different number of either rows or columns.
    pub fn intersect(&mut self, other: &BitSlice2d<'_, R, C>) -> bool {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.columns, other.columns);
        self.words.iter_mut().zip(&*other.words).fold(false, |res, (dst, src)| {
            let prev = *dst;
            *dst &= *src;
            res || prev != *dst
        })
    }
}
impl<R: Idx, C: Idx> BitSlice2d<'_, R, C> {
    /// Creates an iterator which enumerates all rows.
    #[inline]
    #[must_use]
    pub fn enumerate_rows(&self) -> impl ExactSizeIterator<Item = (R, &BitSlice<C>)> + Clone {
        self.words
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    /// Creates an iterator which enumerates all rows.
    #[inline]
    pub fn enumerate_rows_mut(&mut self) -> impl ExactSizeIterator<Item = (R, &mut BitSlice<C>)> {
        self.words
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    /// Gets a reference to the given row.
    ///
    /// # Panics
    /// Panics if the row greater than the number of rows.
    #[inline]
    #[track_caller]
    pub fn row(&self, row: R) -> &BitSlice<C> {
        assert!(row.index() < self.rows as usize);
        let start = self.row_stride as usize * row.index();
        BitSlice::from_words(&self.words[start..start + self.row_stride as usize])
    }

    /// Gets a reference to the given row.
    ///
    /// # Panics
    /// Panics if the row greater than the number of rows.
    #[inline]
    #[track_caller]
    pub fn row_mut(&mut self, row: R) -> &mut BitSlice<C> {
        assert!(row.index() < self.rows as usize);
        let start = self.row_stride as usize * row.index();
        BitSlice::from_words_mut(&mut self.words[start..start + self.row_stride as usize])
    }

    /// Copies a range of rows to another part of the bitset.
    ///
    /// # Panics
    /// Panics if either the source or destination range exceeds the number of rows.
    #[inline]
    #[track_caller]
    pub fn copy_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: WithStride>, dst: R) {
        let src = src.into_slice_idx().with_stride(self.row_stride);
        self.words.copy_within(src, dst.index() * self.row_stride as usize);
    }

    /// Moves a range of rows to another part of the bitset leaving empty rows behind.
    ///
    /// # Panics
    /// Panics if either the source or destination range exceeds the number of rows.
    #[inline]
    #[track_caller]
    pub fn move_rows(
        &mut self,
        src: impl IntoSliceIdx<R, [usize], Output: WithStride<Output: range::Len + SubtractFromEdge>>,
        dst: R,
    ) {
        let src = src.into_slice_idx().with_stride(self.row_stride);
        let dst_start = dst.index() * self.row_stride as usize;
        self.words.copy_within(src.clone(), dst_start);
        let src_len = src.len();
        self.words[src.subtract_from_edge(dst_start..dst_start + src_len)].fill(0);
    }

    /// Clears all elements from a range of rows.
    ///
    /// # Panics
    /// Panics if the range exceeds the number of rows.
    #[inline]
    #[track_caller]
    pub fn clear_rows(&mut self, rows: impl IntoSliceIdx<R, [usize], Output: WithStride>) {
        let words = &mut self.words[rows.into_slice_idx().with_stride(self.row_stride)];
        words.fill(0);
    }
}

impl<R, C> PartialEq for BitSlice2d<'_, R, C> {
    fn eq(&self, other: &Self) -> bool {
        self.columns == other.columns && self.rows == other.rows && self.words == other.words
    }
}
impl<R, C> Eq for BitSlice2d<'_, R, C> {}

/// A two-dimensional bit set with a fixed number of columns and a dynamic number of rows.
///
/// This is represented as a dense array of words stored in row major order with each row aligned to
/// the start of a word. Any row not physically stored will be treated as though it contains no
/// items and storage for the row (and all previous rows) will be allocated as needed to store
/// values. In effect this will behave as though it had the maximum number of rows representable by
/// `R`.
pub struct GrowableBitSet2d<R, C> {
    words: Vec<Word>,
    rows: u32,
    columns: u32,
    row_stride: u32,
    phantom: PhantomData<(R, C)>,
}
impl<R, C> GrowableBitSet2d<R, C> {
    /// Creates a new bit set with the given number of columns without allocating any storage.
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn new(columns: u32) -> Self {
        Self {
            words: Vec::new(),
            rows: 0,
            columns,
            row_stride: word_count_from_bits(columns as usize) as u32,
            phantom: PhantomData,
        }
    }

    /// Gets the number of rows for which values are currently stored.
    #[inline]
    #[must_use]
    pub const fn row_len(&self) -> u32 {
        self.rows
    }

    /// Gets the number of columns.
    #[inline]
    #[must_use]
    pub const fn column_len(&self) -> u32 {
        self.columns
    }

    /// Get the backing slice of currently stored words.
    #[inline]
    #[must_use]
    pub fn words(&self) -> &[Word] {
        self.words.as_slice()
    }

    /// Get the backing slice of currently stored words.
    #[inline]
    #[must_use]
    pub fn words_mut(&mut self) -> &mut [Word] {
        self.words.as_mut_slice()
    }

    /// Checks if the set is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&x| x == 0)
    }

    /// Creates an iterator over a range of stored rows. Any unstored rows within the range will be
    /// silently ignored.
    #[inline]
    #[must_use]
    pub fn iter_rows(
        &self,
        range: impl IntoSliceIdx<R, [usize], Output: LimitExplicitBounds>,
    ) -> impl ExactSizeIterator<Item = &BitSlice<C>> + Clone {
        self.words[range
            .into_slice_idx()
            .limit_explicit_bounds(self.rows as usize)
            .with_stride(self.row_stride)]
        .chunks_exact(self.row_stride as usize)
        .map(|words| BitSlice::from_words(words))
    }

    /// Creates an iterator over a range of stored rows. Any unstored rows within the range will be
    /// silently ignored.
    #[inline]
    #[must_use]
    pub fn iter_mut_rows(
        &mut self,
        range: impl IntoSliceIdx<R, [usize], Output: LimitExplicitBounds>,
    ) -> impl ExactSizeIterator<Item = &mut BitSlice<C>> {
        self.words[range
            .into_slice_idx()
            .limit_explicit_bounds(self.rows as usize)
            .with_stride(self.row_stride)]
        .chunks_exact_mut(self.row_stride as usize)
        .map(|words| BitSlice::from_words_mut(words))
    }

    /// Counts the number of elements in the set.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.words.iter().map(|&x| x.count_ones() as usize).sum()
    }

    /// Removes all items in the set and resets the number of stored rows to zero.
    ///
    /// This will not deallocate any currently allocated storage.
    #[inline]
    pub fn clear(&mut self) {
        self.words.clear();
        self.rows = 0;
    }

    /// Performs a union of two sets storing the result in `self`. Returns `true` if `self` has
    /// changed.
    ///
    /// The number of rows stored in `self` will be extended if needed.
    ///
    /// # Panics
    /// Panics if the sets contain a different number of columns.
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
    /// Creates an iterator which enumerates all stored rows.
    #[inline]
    #[must_use]
    pub fn enumerate_rows(&self) -> impl ExactSizeIterator<Item = (R, &BitSlice<C>)> + Clone {
        self.words
            .chunks_exact(self.row_stride as usize)
            .map(|words| BitSlice::from_words(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    /// Creates an iterator which enumerates all stored rows.
    #[inline]
    #[must_use]
    pub fn enumerate_mut_rows(&mut self) -> impl ExactSizeIterator<Item = (R, &mut BitSlice<C>)> {
        self.words
            .chunks_exact_mut(self.row_stride as usize)
            .map(|words| BitSlice::from_words_mut(words))
            .enumerate()
            .map(|(i, row)| (R::new(i), row))
    }

    /// Gets a reference to a row if the row is stored, or `None` if it is not.
    #[inline]
    pub fn opt_row(&self, row: R) -> Option<&BitSlice<C>> {
        let start = self.row_stride as usize * row.index();
        self.words
            .get(start..start + self.row_stride as usize)
            .map(BitSlice::from_words)
    }

    /// Gets a reference to a row, allocating storage for it if needed.
    ///
    /// This will also allocate storage for all previous rows.
    #[inline]
    #[expect(clippy::cast_possible_truncation)]
    pub fn ensure_row(&mut self, row: R) -> &mut BitSlice<C> {
        let start = self.row_stride as usize * row.index();
        let end = start + self.row_stride as usize;
        BitSlice::from_words_mut(if self.words.get_mut(start..end).is_some() {
            // Can't use the borrow from before due to borrow checking errors.
            &mut self.words[start..end]
        } else {
            self.words.resize(end, 0);
            self.rows = row.index() as u32 + 1;
            &mut self.words[start..end]
        })
    }

    /// Clears all elements from a range of rows.
    ///
    /// Any unstored rows referenced by the range will be silently ignored.
    #[inline]
    pub fn clear_rows(&mut self, rows: impl IntoSliceIdx<R, [usize], Output: LimitExplicitBounds>) {
        self.words[rows
            .into_slice_idx()
            .limit_explicit_bounds(self.rows as usize)
            .with_stride(self.row_stride)]
        .fill(0);
    }

    /// Copies a range of rows to another part of the bitset.
    ///
    /// All unstored rows in the source range will be treated as though they were empty. All
    /// unstored rows in the destination range with a corresponding stored row in the source range
    /// will be allocated.
    #[expect(clippy::cast_possible_truncation)]
    pub fn copy_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: range::SplitAt>, dst: R) {
        let (src_range, src_extra) = src.into_slice_idx().split_at(self.rows as usize);
        let src_row_len = src_range.len();
        if src_row_len == 0 {
            let range = (dst.index()..dst.index() + src_extra)
                .with_stride(self.row_stride)
                .limit_explicit_bounds(self.words.len());
            self.words[range].fill(0);
        } else {
            let dst_row_end = dst.index() + src_row_len;
            let dst_start = dst.index() * self.row_stride as usize;
            let src_range = src_range.with_stride(self.row_stride);
            let dst_copy_end = dst_start + src_range.len();
            if self.rows < dst_row_end as u32 {
                self.words.resize(dst_copy_end, 0);
                self.rows = dst_row_end as u32;
            }
            self.words.copy_within(src_range, dst_start);
            let dst_end = self
                .words
                .len()
                .min(dst_copy_end + src_extra * self.row_stride as usize);
            self.words[dst_copy_end..dst_end].fill(0);
        }
    }

    /// Moves a range of rows to another part of the bitset leaving empty rows behind.
    ///
    /// All unstored rows in the source range will be treated as though they were empty. All
    /// unstored rows in the destination range with a corresponding stored row in the source range
    /// will be allocated.
    #[expect(clippy::cast_possible_truncation)]
    pub fn move_rows(&mut self, src: impl IntoSliceIdx<R, [usize], Output: range::SplitAt>, dst: R) {
        let (src_range, src_extra) = src.into_slice_idx().split_at(self.rows as usize);
        let src_row_len = src_range.len();
        if src_row_len == 0 {
            let range = (dst.index()..dst.index() + src_extra)
                .with_stride(self.row_stride)
                .limit_explicit_bounds(self.words.len());
            self.words[range].fill(0);
        } else {
            let dst_row_end = dst.index() + src_row_len;
            let dst_start = dst.index() * self.row_stride as usize;
            let src_range = src_range.with_stride(self.row_stride);
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
            self.words[src_range.subtract_from_edge(dst_start..dst_end)].fill(0);
        }
    }
}

impl<R, C> PartialEq for GrowableBitSet2d<R, C> {
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(self.columns, other.columns);
        let (lhs, rhs, extra) = if let Some((lhs, extra)) = self.words.split_at_checked(other.words.len()) {
            (lhs, other.words.as_slice(), extra)
        } else {
            let (rhs, extra) = other.words.split_at(self.words.len());
            (self.words.as_slice(), rhs, extra)
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

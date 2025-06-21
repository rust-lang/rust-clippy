use core::marker::PhantomData;
use core::mem::{self, transmute};
use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::slice::{self, SliceIndex};
use core::{iter, range};
use rustc_arena::DroplessArena;
use rustc_index::{Idx, IntoSliceIdx};

pub type Word = usize;
pub const WORD_BITS: usize = Word::BITS as usize;
pub const MAX_WORDS: usize = usize::MAX / WORD_BITS;

#[inline]
pub const fn word_count_from_bits(bits: usize) -> usize {
    (bits + (WORD_BITS - 1)) / WORD_BITS
}

#[inline]
pub const fn final_mask_for_size(bits: usize) -> Word {
    (!(!0usize << bits % WORD_BITS)).wrapping_sub((bits % WORD_BITS == 0) as usize)
}

pub struct BitRange<R> {
    /// The range of affected words.
    words: R,
    /// The amount to shift to make a bit-mask for the first word.
    first_shift: u8,
    /// The amount to shift to make a bit-mask for the last word.
    last_shift: u8,
}
impl<R> BitRange<R> {
    #[inline]
    pub const fn first_mask(&self) -> Word {
        !0usize << self.first_shift
    }

    #[inline]
    pub const fn last_mask(&self) -> Word {
        !0usize >> self.last_shift
    }
}

pub trait IntoBitRange: Sized {
    type Range: SliceIndex<[Word], Output = [Word]>;
    fn into_bit_range(self) -> BitRange<Self::Range>;
}
impl IntoBitRange for RangeFull {
    type Range = Self;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self> {
        BitRange {
            words: self,
            first_shift: 0,
            last_shift: 0,
        }
    }
}
impl IntoBitRange for Range<usize> {
    type Range = Self;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self> {
        let start = BitIdx::from_bit(self.start);
        let end = BitIdx::from_bit(self.end);
        BitRange {
            words: Range {
                start: start.word,
                end: end.word + (end.bit != 0) as usize,
            },
            first_shift: start.bit as u8,
            last_shift: ((WORD_BITS - 1) - (end.bit.wrapping_sub(1) % WORD_BITS)) as u8,
        }
    }
}
impl IntoBitRange for RangeFrom<usize> {
    type Range = Self;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self> {
        let start = BitIdx::from_bit(self.start);
        BitRange {
            words: RangeFrom { start: start.word },
            first_shift: start.bit as u8,
            last_shift: 0,
        }
    }
}
impl IntoBitRange for RangeTo<usize> {
    type Range = Self;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self> {
        let end = BitIdx::from_bit(self.end);
        BitRange {
            words: RangeTo {
                end: end.word + (end.bit != 0) as usize,
            },
            first_shift: 0,
            last_shift: ((WORD_BITS - 1) - (end.bit.wrapping_sub(1) % WORD_BITS)) as u8,
        }
    }
}
impl IntoBitRange for RangeInclusive<usize> {
    type Range = Range<usize>;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self::Range> {
        let start = BitIdx::from_bit(*self.start());
        let end = BitIdx::from_bit(*self.end());
        BitRange {
            words: Range {
                start: start.word,
                end: end.word + 1,
            },
            first_shift: start.bit as u8,
            last_shift: ((WORD_BITS - 1) - end.bit) as u8,
        }
    }
}
impl IntoBitRange for RangeToInclusive<usize> {
    type Range = RangeTo<usize>;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self::Range> {
        let end = BitIdx::from_bit(self.end);
        BitRange {
            words: RangeTo { end: end.word + 1 },
            first_shift: 0,
            last_shift: ((WORD_BITS - 1) - end.bit) as u8,
        }
    }
}
impl IntoBitRange for range::Range<usize> {
    type Range = range::Range<usize>;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self::Range> {
        let start = BitIdx::from_bit(self.start);
        let end = BitIdx::from_bit(self.end);
        BitRange {
            words: range::Range {
                start: start.word,
                end: end.word + (end.bit != 0) as usize,
            },
            first_shift: start.bit as u8,
            last_shift: ((WORD_BITS - 1) - (end.bit.wrapping_sub(1) % WORD_BITS)) as u8,
        }
    }
}
impl IntoBitRange for range::RangeFrom<usize> {
    type Range = range::RangeFrom<usize>;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self::Range> {
        let start = BitIdx::from_bit(self.start);
        BitRange {
            words: range::RangeFrom { start: start.word },
            first_shift: start.bit as u8,
            last_shift: 0,
        }
    }
}
impl IntoBitRange for range::RangeInclusive<usize> {
    type Range = range::Range<usize>;
    #[inline]
    fn into_bit_range(self) -> BitRange<Self::Range> {
        let start = BitIdx::from_bit(self.start);
        let end = BitIdx::from_bit(self.end);
        BitRange {
            words: range::Range {
                start: start.word,
                end: end.word + 1,
            },
            first_shift: start.bit as u8,
            last_shift: ((WORD_BITS - 1) - end.bit) as u8,
        }
    }
}

struct BitIdx {
    word: usize,
    bit: usize,
}
impl BitIdx {
    #[inline]
    fn from_bit<T: Idx>(bit: T) -> Self {
        let bit = bit.index();
        Self {
            word: bit / WORD_BITS,
            bit: bit % WORD_BITS,
        }
    }

    #[inline]
    fn word_mask(&self) -> Word {
        1usize << self.bit
    }
}

/// A slice of words where each bit is treated as an individual value.
///
/// Note: This can only hold bits as a multiple of `WORD_SIZE`. Use
/// `mask_final_word(final_mask_for_size(len))` to clear the final bits greater than or equal to
/// `len`.
#[repr(transparent)]
pub struct BitSlice<T> {
    phantom: PhantomData<T>,
    pub words: [Word],
}
impl<T> BitSlice<T> {
    #[inline]
    pub const fn from_words(words: &[Word]) -> &Self {
        debug_assert!(words.len() <= MAX_WORDS);
        // SAFETY: `BitSlice` is a transparent wrapper around `[Word]`.
        unsafe { transmute::<&[Word], &Self>(words) }
    }

    #[inline]
    pub fn from_words_mut(words: &mut [Word]) -> &mut Self {
        debug_assert!(words.len() <= MAX_WORDS);
        // SAFETY: `BitSlice` is a transparent wrapper around `[Word]`.
        unsafe { transmute::<&mut [Word], &mut Self>(words) }
    }

    #[inline]
    pub fn from_boxed_words(words: Box<[Word]>) -> Box<Self> {
        debug_assert!(words.len() <= MAX_WORDS);
        // SAFETY: `BitSlice` is a transparent wrapper around `[Word]`.
        unsafe { transmute::<Box<[Word]>, Box<Self>>(words) }
    }

    /// Gets the size of this slice in bits.
    #[inline]
    pub const fn bit_len(&self) -> usize {
        self.words.len() * WORD_BITS
    }

    /// Checks whether all bits are zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&x| x == 0)
    }

    /// Counts the number of set bits in the slice.
    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|&x| x.count_ones() as usize).sum()
    }

    /// Allocates a boxed `BitSlice` of the given size rounded up to the nearest word size.
    #[inline]
    pub fn empty_box(bits: usize) -> Box<Self> {
        Self::from_boxed_words(vec![0usize; word_count_from_bits(bits)].into_boxed_slice())
    }

    /// Allocates a `BitSlice` of the given size rounded up to the nearest word size.
    #[inline]
    pub fn empty_arena(arena: &DroplessArena, bits: usize) -> &mut Self {
        Self::from_words_mut(arena.alloc_from_iter(iter::repeat_n(0, word_count_from_bits(bits))))
    }

    /// Applies a bit-mask to the final word of the slice.
    #[inline]
    pub fn mask_final_word(&mut self, mask: Word) {
        if let Some(word) = self.words.last_mut() {
            *word &= mask;
        }
    }

    /// Fills the entire bit slice.
    ///
    /// Note: This can only work with whole `Word`s. Use `mask_final_word(final_mask_for_size(len))`
    /// to clear the final bits greater than or equal to `len`.
    #[inline]
    pub fn fill(&mut self) {
        self.words.fill(!0);
    }

    /// Clears the entire bit slice.
    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Calculates the union over two sets storing the results in `self`. Return if `self` has
    /// changed.
    ///
    /// Note: The result will be truncated to the number of bits contained in `self`
    pub fn union_trunc(&mut self, other: &Self) -> bool {
        self.words.iter_mut().zip(&other.words).fold(false, |res, (lhs, rhs)| {
            let prev = *lhs;
            *lhs |= *rhs;
            prev != *lhs || res
        })
    }

    /// Calculates the intersection over two sets storing the results in `self`. Return if `self`
    /// has changed.
    pub fn intersect(&mut self, other: &Self) -> bool {
        self.words.iter_mut().zip(&other.words).fold(false, |res, (lhs, rhs)| {
            let prev = *lhs;
            *lhs &= *rhs;
            prev != *lhs || res
        })
    }

    /// Subtracts `other` from `self` storing the results in `self`. Return if `self` has changed.
    pub fn subtract(&mut self, other: &Self) -> bool {
        self.words.iter_mut().zip(&other.words).fold(false, |res, (lhs, rhs)| {
            let prev = *lhs;
            *lhs &= !*rhs;
            prev != *lhs || res
        })
    }
}
impl<T: Idx> BitSlice<T> {
    /// Inserts the given bit and returns whether the value changed.
    #[inline]
    #[track_caller]
    pub fn insert(&mut self, bit: T) -> bool {
        let idx = BitIdx::from_bit(bit);
        let res = self.words[idx.word] & idx.word_mask() == 0;
        self.words[idx.word] |= idx.word_mask();
        res
    }

    /// Checks if the given bit is contained in the slice.
    #[inline]
    pub fn contains(&self, bit: T) -> bool {
        let idx = BitIdx::from_bit(bit);
        self.words.get(idx.word).map_or(0, |&x| x) & idx.word_mask() != 0
    }

    /// Inserts the given range of bits into the slice.
    #[track_caller]
    pub fn insert_range(&mut self, range: impl IntoSliceIdx<T, [Word], Output: IntoBitRange>) {
        let range = range.into_slice_idx().into_bit_range();
        let first = range.first_mask();
        let last = range.last_mask();
        match &mut self.words[range.words] {
            [] => {},
            [word] => *word |= first & last,
            [word1, words @ .., word2] => {
                *word1 |= first;
                words.fill(!0);
                *word2 |= last;
            },
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(&self.words)
    }

    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain::new(&mut self.words)
    }
}

impl<T: Idx> Extend<T> for &mut BitSlice<T> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        for i in iter {
            self.insert(i);
        }
    }
}

#[derive(Default, Clone)]
pub struct WordBitIter(Word);
impl WordBitIter {
    #[inline]
    pub const fn new(word: Word) -> Self {
        Self(word)
    }
}
impl Iterator for WordBitIter {
    type Item = u32;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let bit_pos = self.0.trailing_zeros();
            self.0 ^= 1 << bit_pos;
            Some(bit_pos)
        }
    }
}

// `BitIter` copied from rustc.
pub struct Iter<'a, T: Idx> {
    /// Iterator over a single word.
    word: WordBitIter,

    /// The offset (measured in bits) of the current word.
    offset: usize,

    /// Underlying iterator over the words.
    iter: slice::Iter<'a, Word>,

    marker: PhantomData<T>,
}
impl<'a, T: Idx> Iter<'a, T> {
    #[inline]
    fn new(words: &'a [Word]) -> Self {
        // We initialize `word` and `offset` to degenerate values. On the first
        // call to `next()` we will fall through to getting the first word from
        // `iter`, which sets `word` to the first word (if there is one) and
        // `offset` to 0. Doing it this way saves us from having to maintain
        // additional state about whether we have started.
        Self {
            word: WordBitIter::new(0),
            offset: usize::MAX - (WORD_BITS - 1),
            iter: words.iter(),
            marker: PhantomData,
        }
    }
}
impl<'a, T: Idx> Iterator for Iter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        loop {
            if let Some(idx) = self.word.next() {
                return Some(T::new(idx as usize + self.offset));
            }

            // Move onto the next word. `wrapping_add()` is needed to handle
            // the degenerate initial value given to `offset` in `new()`.
            self.word = WordBitIter::new(*self.iter.next()?);
            self.offset = self.offset.wrapping_add(WORD_BITS);
        }
    }
}

pub struct Drain<'a, T> {
    word: WordBitIter,
    offset: usize,
    iter: slice::IterMut<'a, usize>,
    marker: PhantomData<T>,
}
impl<'a, T> Drain<'a, T> {
    #[inline]
    fn new(words: &'a mut [Word]) -> Self {
        Self {
            word: WordBitIter::new(0),
            offset: usize::MAX - (WORD_BITS - 1),
            iter: words.iter_mut(),
            marker: PhantomData,
        }
    }
}
impl<T> Drop for Drain<'_, T> {
    #[inline]
    fn drop(&mut self) {
        for x in &mut self.iter {
            *x = 0;
        }
    }
}
impl<T: Idx> Iterator for Drain<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        loop {
            if let Some(idx) = self.word.next() {
                return Some(T::new(idx as usize + self.offset));
            }

            // Move onto the next word. `wrapping_add()` is needed to handle
            // the degenerate initial value given to `offset` in `new()`.
            self.word = WordBitIter::new(mem::replace(self.iter.next()?, 0));
            self.offset = self.offset.wrapping_add(WORD_BITS);
        }
    }
}

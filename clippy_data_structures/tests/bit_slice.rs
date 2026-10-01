#![feature(rustc_private)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::range_minus_one
)]

use clippy_data_structures::bit_slice::{BitSlice, WORD_BITS};

#[test]
fn union_intersect_subtract() {
    let mut x = [0b10101010, 0b10101010];
    let mut y = [0b01010101, 0b11110000];
    let x = BitSlice::<usize>::from_words_mut(&mut x);
    let y = BitSlice::<usize>::from_words_mut(&mut y);

    assert!(x.union_trunc(y));
    assert_eq!(&x.words, &[0b11111111, 0b11111010]);
    assert!(!x.union_trunc(y));
    assert_eq!(&x.words, &[0b11111111, 0b11111010]);
    assert!(x.subtract(y));
    assert_eq!(&x.words, &[0b10101010, 0b00001010]);
    assert!(!x.subtract(y));
    assert_eq!(&x.words, &[0b10101010, 0b00001010]);

    assert!(x.union_trunc(y));
    assert_eq!(&x.words, &[0b11111111, 0b11111010]);
    assert!(x.intersect(y));
    assert_eq!(&x.words, &[0b01010101, 0b11110000]);
    assert!(!x.intersect(y));
    assert_eq!(&x.words, &[0b01010101, 0b11110000]);

    x.clear();
    assert!(x.is_empty());
    assert!(!x.subtract(y));
    assert!(!x.intersect(y));
    assert!(x.union_trunc(y));
    assert!(x.words == y.words);
}

#[test]
fn insert_range() {
    let mut x = [0, 0, 0];
    let x = BitSlice::<usize>::from_words_mut(&mut x);

    x.insert_range(0..WORD_BITS);
    assert_eq!(&x.words, &[!0, 0, 0]);

    x.insert_range(1..=WORD_BITS);
    assert_eq!(&x.words, &[!0, 1, 0]);

    x.insert_range(..WORD_BITS + 3);
    assert_eq!(&x.words, &[!0, 0b111, 0]);

    x.insert_range(..=WORD_BITS * 2 - 1);
    assert_eq!(&x.words, &[!0, !0, 0]);

    x.insert_range(WORD_BITS * 2 + 1..);
    assert_eq!(&x.words, &[!0, !0, !1]);

    x.clear();
    x.insert_range(WORD_BITS / 2..WORD_BITS * 2 + WORD_BITS / 2);
    assert_eq!(&x.words, &[!0 << (WORD_BITS / 2), !0, !0 >> (WORD_BITS / 2)]);

    x.clear();
    x.insert_range(0..0);
    assert_eq!(&x.words, &[0, 0, 0]);

    x.insert_range(1..WORD_BITS - 1);
    assert_eq!(&x.words, &[!1 & (!0 >> 1), 0, 0]);
}

#[test]
fn iter_insert_remove_contains() {
    let mut x = [0, 0, 0];
    let x = BitSlice::<usize>::from_words_mut(&mut x);

    assert!(x.iter().eq::<[usize; 0]>([]));

    assert!(!x.contains(1));
    assert!(x.insert(1));
    assert!(x.contains(1));
    assert!(x.iter().eq([1]));

    assert!(!x.contains(2));
    assert!(x.insert(2));
    assert!(x.contains(2));
    assert!(x.iter().eq([1, 2]));

    assert!(!x.contains(0));
    assert!(x.insert(0));
    assert!(x.contains(0));
    assert!(x.iter().eq([0, 1, 2]));

    assert!(!x.contains(WORD_BITS));
    assert!(x.insert(WORD_BITS));
    assert!(x.contains(WORD_BITS));
    assert!(x.iter().eq([0, 1, 2, WORD_BITS]));

    assert!(!x.contains(WORD_BITS * 2 + 1));
    assert!(x.insert(WORD_BITS * 2 + 1));
    assert!(x.contains(WORD_BITS * 2 + 1));
    assert!(x.iter().eq([0, 1, 2, WORD_BITS, WORD_BITS * 2 + 1]));

    assert!(!x.insert(0));
    assert!(x.iter().eq([0, 1, 2, WORD_BITS, WORD_BITS * 2 + 1]));

    assert!(x.remove(0));
    assert!(!x.contains(0));
    assert!(x.iter().eq([1, 2, WORD_BITS, WORD_BITS * 2 + 1]));

    assert!(!x.remove(0));
    assert!(x.iter().eq([1, 2, WORD_BITS, WORD_BITS * 2 + 1]));

    assert!(!x.contains(WORD_BITS * 2));
    assert!(x.insert(WORD_BITS * 2));
    assert!(x.contains(WORD_BITS * 2));
    assert!(x.drain().eq([1, 2, WORD_BITS, WORD_BITS * 2, WORD_BITS * 2 + 1]));
    assert!(x.is_empty());
}

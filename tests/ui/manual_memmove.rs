#![warn(clippy::manual_memmove)]

use std::ops::{Index, IndexMut};

const OFFSET: usize = 4;
const NEGATIVE_OFFSET: isize = 4;

pub fn manual_memmove(arr: &mut [u8]) {
    for i in 0..(arr.len() - 4) {
        // left shift by 4
        arr[i] = arr[i + 4];
    }

    for i in 0..(arr.len() - 4) {
        // left shift by 4 with more type inference
        arr[i] = arr[i + OFFSET];
    }

    for i in (0..(arr.len() - 4)).rev() {
        // right shift by 4, currently not supported
        arr[i] = arr[i - 4];
    }

    for i in 0..(arr.len() - 4) {
        // right shift by 4 with more type inference, currently not supported
        arr[i] = arr[i - OFFSET];
    }

    for i in 0..(arr.len() - 4) {
        // right shift by 4 with more type inference (alt version), currently not supported
        arr[i] = arr[i + NEGATIVE_OFFSET as usize];
    }

    for i in 0..arr.len() {
        // left shift by 4, currently not supported
        arr[i] = arr[i + 4];
        if i >= (arr.len() - 4) {
            break;
        }
    }

    for i in 10..256 {
        // multiple memmoves, currently not supported
        arr[i] = arr[i + 4];
        arr[i + 500] = arr[i + 508];
    }

    for i in 10..256 {
        // not sure what sense this would make but that's not a memmove
        arr[i + 4] = arr[i];
    }

    for i in 10..256 {
        // not sure what sense this would make but that's not a memmove either
        arr[i] = arr[i - 1];
    }

    let mut arr = vec![1, 2, 3, 4, 5];

    for i in 0..(arr.len() - 4) {
        // make sure vectors are supported
        arr[i] = arr[i + 4];
    }

    struct DummyStruct(u8);

    impl Index<usize> for DummyStruct {
        type Output = u8;
        fn index(&self, _: usize) -> &u8 {
            &self.0
        }
    }

    impl IndexMut<usize> for DummyStruct {
        fn index_mut(&mut self, _: usize) -> &mut u8 {
            &mut self.0
        }
    }

    let mut arr = DummyStruct(0);

    for i in 0..4 {
        // lint should not trigger when `arr` is not slice-like, like DummyStruct
        arr[i] = arr[i + 4];
    }

    let mut arr = std::collections::VecDeque::from_iter([0; 5]);
    for i in 0..(arr.len() - 4) {
        // VecDeque - ideally this should work
        arr[i] = arr[i + 4];
    }
}

#[warn(clippy::needless_range_loop, clippy::manual_memmove)]
pub fn manual_clone(arr: &mut [String]) {
    for i in 0..arr.len() {
        // should not suggest for non-copy items
        arr[i] = arr[i + 1].clone();
    }
}

fn main() {}

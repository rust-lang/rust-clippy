#![warn(clippy::chunks_exact_with_const_size)]
#![allow(unused)]

fn main() {
    let slice = [1, 2, 3, 4, 5, 6, 7, 8];
    const CHUNK_SIZE: usize = 4;

    // Should trigger lint with help message only (not suggestion) - stored in variable
    let mut chunk_iter = slice.chunks_exact(CHUNK_SIZE);
    //~^ chunks_exact_with_const_size
    for chunk in chunk_iter.by_ref() {}
    let _remainder = chunk_iter.remainder();

    // Similar for mutable version - help message only
    let mut arr2 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut chunk_iter = arr2.chunks_exact_mut(CHUNK_SIZE);
    //~^ chunks_exact_with_const_size
    for chunk in chunk_iter.by_ref() {}
    let _remainder = chunk_iter.into_remainder();
}

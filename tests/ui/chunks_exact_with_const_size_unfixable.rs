#![warn(clippy::chunks_exact_with_const_size)]
#![allow(unused)]
#![allow(clippy::iter_cloned_collect)]

fn main() {
    let slice = [1, 2, 3, 4, 5, 6, 7, 8];
    const CHUNK_SIZE: usize = 4;

    // Should trigger lint - literal constant (stored in let binding, so help not suggestion)
    let result = slice.chunks_exact(4);
    //~^ chunks_exact_with_const_size

    // Should trigger lint - const value
    let result = slice.chunks_exact(CHUNK_SIZE);
    //~^ chunks_exact_with_const_size

    // Should trigger lint - simple iteration
    let result = slice.chunks_exact(3);
    //~^ chunks_exact_with_const_size

    // Should trigger - mutable variant
    let mut arr = [1, 2, 3, 4, 5, 6, 7, 8];
    let result = arr.chunks_exact_mut(4);
    //~^ chunks_exact_with_const_size

    // Should trigger - multiline expression
    #[rustfmt::skip]
    let result = slice
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .chunks_exact(2);
    //~^ chunks_exact_with_const_size

    // Should trigger - array coerces to slice reference
    let array = [1, 2, 3, 4, 5, 6, 7, 8];
    let result = array.chunks_exact(4);
    //~^ chunks_exact_with_const_size

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

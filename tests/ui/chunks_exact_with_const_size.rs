#![warn(clippy::chunks_exact_with_const_size)]
#![allow(unused)]
#![allow(clippy::iter_cloned_collect)]

fn main() {
    let slice = [1, 2, 3, 4, 5, 6, 7, 8];

    // Should trigger lint - literal constant
    let result = slice.chunks_exact(4);
    //~^ chunks_exact_with_const_size

    // Should trigger lint - const value
    const CHUNK_SIZE: usize = 4;
    let result = slice.chunks_exact(CHUNK_SIZE);
    //~^ chunks_exact_with_const_size

    // Should NOT trigger - runtime value
    let size = 4;
    let mut it = slice.chunks_exact(size);
    for chunk in it {}

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
}

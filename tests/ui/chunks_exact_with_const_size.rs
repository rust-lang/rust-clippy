#![warn(clippy::chunks_exact_with_const_size)]
#![allow(unused)]

fn main() {
    let slice = [1, 2, 3, 4, 5, 6, 7, 8];

    // Should NOT trigger - runtime value
    let size = 4;
    let mut it = slice.chunks_exact(size);
    for chunk in it {}

    // Should trigger - direct iteration without binding (gets suggestion)
    for chunk in slice.chunks_exact(4) {
        //~^ chunks_exact_with_const_size
        let _ = chunk;
    }

    // Should trigger - direct iteration with const
    const CHUNK_SIZE: usize = 4;
    for chunk in slice.chunks_exact(CHUNK_SIZE) {
        //~^ chunks_exact_with_const_size
        let _ = chunk;
    }

    // Should trigger - chunks_exact_mut with direct iteration (gets suggestion without .iter())
    let mut arr = [1, 2, 3, 4, 5, 6, 7, 8];
    for chunk in arr.chunks_exact_mut(4) {
        //~^ chunks_exact_with_const_size
        let _ = chunk;
    }

    // Should trigger - used with iterator method (not for loop, so needs .iter())
    let _: Vec<_> = slice.chunks_exact(4).collect();
    //~^ chunks_exact_with_const_size

    // Should NOT trigger - macro-expanded sizes are not recognized as const by is_const_evaluatable
    macro_rules! chunk_size {
        () => {
            4
        };
    }
    for chunk in slice.chunks_exact(chunk_size!()) {
        let _ = chunk;
    }
}

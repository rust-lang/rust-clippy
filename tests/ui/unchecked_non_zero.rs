#![warn(clippy::unchecked_non_zero)]
#![allow(unused_must_use, clippy::boxed_local, clippy::useless_vec)]

use std::num::NonZero;

// --- slices: `chunk size must be non-zero` / `window size must be non-zero` ---

fn slices_known_non_zero(data: &[u8], v: Vec<u8>, arr: [u8; 8], boxed: Box<[u8]>) {
    const SIZE: usize = 4;

    // Literals and constants are proof enough.
    data.chunks(2);
    data.windows(1);
    data.chunks(SIZE);
    data.chunks(SIZE * 2);

    // Autoderef targets are still slices.
    v.chunks(2);
    arr.chunks(2);
    boxed.windows(2);
}

fn slices_non_zero_type(data: &[u8], size: NonZero<usize>) {
    data.chunks(size.get());
    data.windows(size.get());
}

fn slices_guarded_by_max(data: &[u8], size: usize) {
    data.chunks(size.max(1));
    data.windows(size.max(2));
}

fn slices_unknown(data: &[u8], size: usize) {
    data.chunks(size);
    //~^ unchecked_non_zero
    data.windows(size);
    //~^ unchecked_non_zero
    data.chunks_exact(size);
    //~^ unchecked_non_zero
    data.rchunks(size);
    //~^ unchecked_non_zero
    data.rchunks_exact(size);
    //~^ unchecked_non_zero
    data.chunks(size / 2);
    //~^ unchecked_non_zero
    data.chunks(std::env::args().count());
    //~^ unchecked_non_zero
}

fn slices_unknown_mut(data: &mut [u8], size: usize) {
    data.chunks_mut(size);
    //~^ unchecked_non_zero
    data.chunks_exact_mut(size);
    //~^ unchecked_non_zero
    data.rchunks_mut(size);
    //~^ unchecked_non_zero
    data.rchunks_exact_mut(size);
    //~^ unchecked_non_zero
}

fn slices_statically_zero(data: &[u8]) {
    const ZERO: usize = 0;

    data.chunks(0);
    //~^ unchecked_non_zero
    data.windows(ZERO);
    //~^ unchecked_non_zero
}

// --- `Iterator::step_by` asserts `step != 0` ---

fn step_by(size: usize, non_zero: NonZero<usize>) {
    (0..10).step_by(2);
    (0..10).step_by(non_zero.get());
    (0..10).step_by(size.max(1));

    (0..10).step_by(size);
    //~^ unchecked_non_zero

    // A literal `0` belongs to `iterator_step_by_zero`, so this lint stays quiet.
    #[allow(clippy::iterator_step_by_zero)]
    let _ = (0..10).step_by(0);
}

// --- `ilog2` / `ilog10` / `ilog` panic on a non-positive receiver ---

fn ilog_known_valid(non_zero: NonZero<u32>) {
    7u32.ilog2();
    100u32.ilog10();
    7u32.ilog(3);
    non_zero.get().ilog2();
    // `NonZero`'s own `ilog2` cannot panic at all.
    non_zero.ilog2();
}

fn ilog_unknown(x: u32, base: u32) {
    x.ilog2();
    //~^ unchecked_non_zero
    x.ilog10();
    //~^ unchecked_non_zero
    x.ilog(3);
    //~^ unchecked_non_zero

    // The receiver is fine here, but the base is not.
    7u32.ilog(base);
    //~^ unchecked_non_zero
    7u32.ilog(1);
    //~^ unchecked_non_zero
}

fn ilog_signed(x: i32, non_zero: NonZero<i32>) {
    5i32.ilog2();

    x.ilog2();
    //~^ unchecked_non_zero
    // A signed `NonZero` can still be negative.
    non_zero.get().ilog2();
    //~^ unchecked_non_zero
}

fn ilog_statically_invalid() {
    0u32.ilog2();
    //~^ unchecked_non_zero
    (-1i32).ilog2();
    //~^ unchecked_non_zero
}

// --- receivers that are not the std methods ---

fn not_the_std_methods() {
    struct Grid;
    impl Grid {
        fn chunks(&self, _n: usize) {}
        fn ilog2(&self) {}
    }
    Grid.chunks(0);
    Grid.ilog2();
}

fn in_a_macro(data: &[u8], size: usize) {
    macro_rules! chunk {
        ($d:expr, $s:expr) => {
            $d.chunks($s)
        };
    }
    // The call is not written by the user, so it is not linted.
    chunk!(data, size);
}

fn main() {}

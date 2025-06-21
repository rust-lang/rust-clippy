#![warn(clippy::const_sized_chunks_exact)]
#![feature(array_chunks)]
#![allow(unused_variables)]
#![allow(dead_code)]

fn test_binding() {
    let numbers = [0, 1, 2, 3, 4];

    #[allow(clippy::needless_borrow)]
    for ref chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    #[allow(unused_mut)]
    for mut chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    for ref mut chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }
}

fn test_slice_like() {
    let numbers = [2; 5];
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = [0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = &[0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = &&[0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = &&&[0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }

    let numbers = &Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact(2) {
        //~^ const_sized_chunks_exact
    }
}

fn test_const_eval() {
    const N: usize = 2;

    let numbers = [2; 5];
    for chunk in numbers.chunks_exact(N) {
        //~^ const_sized_chunks_exact
    }

    for chunk in numbers.chunks_exact(N + 1) {
        //~^ const_sized_chunks_exact
    }
}

fn test_chunk_size() {
    let numbers = Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact(0) {}

    for chunk in numbers.chunks_exact(1) {
        //~^ const_sized_chunks_exact
    }

    for chunk in numbers.chunks_exact(26) {
        //~^ const_sized_chunks_exact
    }

    for chunk in numbers.chunks_exact(27) {}
}

fn main() {}

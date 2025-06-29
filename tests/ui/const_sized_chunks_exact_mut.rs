#![warn(clippy::const_sized_chunks_exact_mut)]
#![feature(array_chunks)]
#![allow(unused_variables)]
#![allow(dead_code)]

fn test_binding() {
    let mut numbers = [0, 1, 2, 3, 4];

    #[allow(clippy::needless_borrow)]
    for ref chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    #[allow(unused_mut)]
    for mut chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    for ref mut chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }
}

fn test_slice_like() {
    let mut numbers = [2; 5];
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = [0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = &mut [0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = &mut &mut [0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = &mut &mut &mut [0, 1, 2, 3, 4];
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }

    let mut numbers = &mut Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact_mut(2) {
        //~^ const_sized_chunks_exact_mut
    }
}

fn test_const_eval() {
    const N: usize = 2;

    let mut numbers = [2; 5];
    for chunk in numbers.chunks_exact_mut(N) {
        //~^ const_sized_chunks_exact_mut
    }

    for chunk in numbers.chunks_exact_mut(N + 1) {
        //~^ const_sized_chunks_exact_mut
    }
}

fn test_chunk_size() {
    let mut numbers = Vec::from_iter(0..5);
    for chunk in numbers.chunks_exact_mut(0) {}

    for chunk in numbers.chunks_exact_mut(1) {
        //~^ const_sized_chunks_exact_mut
    }

    for chunk in numbers.chunks_exact_mut(26) {
        //~^ const_sized_chunks_exact_mut
    }

    for chunk in numbers.chunks_exact_mut(27) {}
}

fn main() {}

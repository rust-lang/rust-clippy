#![warn(clippy::const_sized_windows)]
#![feature(array_windows)]
#![allow(unused_variables)]
#![allow(dead_code)]

fn test_binding() {
    let numbers = [0, 1, 2, 3, 4];

    #[allow(clippy::needless_borrow)]
    for ref chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    #[allow(unused_mut)]
    for mut chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    for ref mut chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }
}

fn test_slice_like() {
    let numbers = [2; 5];
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = [0, 1, 2, 3, 4];
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = &[0, 1, 2, 3, 4];
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = &&[0, 1, 2, 3, 4];
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = &&&[0, 1, 2, 3, 4];
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = Vec::from_iter(0..5);
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }

    let numbers = &Vec::from_iter(0..5);
    for chunk in numbers.windows(2) {
        //~^ const_sized_windows
    }
}

fn test_const_eval() {
    const N: usize = 2;

    let numbers = [2; 5];
    for chunk in numbers.windows(N) {
        //~^ const_sized_windows
    }

    for chunk in numbers.windows(N + 1) {
        //~^ const_sized_windows
    }
}

fn test_chunk_size() {
    let numbers = Vec::from_iter(0..5);
    for chunk in numbers.windows(0) {}

    for chunk in numbers.windows(1) {
        //~^ const_sized_windows
    }

    for chunk in numbers.windows(26) {
        //~^ const_sized_windows
    }

    for chunk in numbers.windows(27) {}
}

fn main() {}

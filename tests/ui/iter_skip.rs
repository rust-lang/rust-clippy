//@run-rustfix
#![warn(clippy::iter_skip)]
#![allow(
    clippy::iter_skip_next,
    clippy::useless_vec,
    clippy::into_iter_on_ref,
    for_loops_over_fallibles,
    clippy::iter_next_loop
)]
#![no_main]

pub fn _1(list: &[u32]) {
    for _ in list.iter().skip(5) {}
}

pub fn _2(list: &mut [u32]) {
    for _ in list.iter_mut().skip(5) {}
}

pub fn _3(list: Vec<u32>) {
    for _ in list.iter().skip(5) {}
}

pub fn _4(mut list: Vec<u32>) {
    for _ in list.iter_mut().skip(5) {}
}

pub fn _5(list: &[u32]) {
    for _ in list.into_iter().skip(5) {}
}

pub fn _6(list: &mut [u32]) {
    for _ in list.into_iter().skip(5) {}
}

// Don't lint

pub fn _7(list: &[u32]) {
    for _ in list.iter().skip(5).next() {}
}

pub fn _8(list: &[u32]) {
    for _ in list.iter().skip(5).next() {}
}

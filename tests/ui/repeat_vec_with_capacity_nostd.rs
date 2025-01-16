#![warn(clippy::repeat_vec_with_capacity)]
#![no_std]
use core::iter;
extern crate alloc;
use alloc::vec::Vec;

fn nostd() {
    let _: Vec<Vec<u8>> = iter::repeat(Vec::with_capacity(42)).take(123).collect();
}

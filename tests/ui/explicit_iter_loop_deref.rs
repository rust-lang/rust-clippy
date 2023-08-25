#![deny(clippy::explicit_iter_loop_deref)]
#![allow(
    clippy::linkedlist,
    clippy::similar_names,
    clippy::needless_borrow,
    clippy::deref_addrof,
    dead_code
)]

use core::slice;
use std::collections::*;

fn main() {
    let mut vec = vec![1, 2, 3, 4];

    let rmvec = &mut vec;
    for _ in rmvec.iter() {}
    for _ in rmvec.iter_mut() {}

    for _ in &vec {} // these are fine
    for _ in &mut vec {} // these are fine

    for _ in (&mut [1, 2, 3]).iter() {}
}

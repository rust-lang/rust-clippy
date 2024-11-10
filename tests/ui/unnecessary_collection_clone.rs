#![warn(clippy::unnecessary_collection_clone)]
#![allow(clippy::iter_cloned_collect)]

use std::collections::LinkedList;
use std::marker::PhantomData;

fn basic(val: Vec<u8>) -> Vec<u8> {
    val.clone().into_iter().collect()
    //~^ error: using clone on collection to own iterated items
}

fn non_deref_to_slice(val: LinkedList<u8>) -> Vec<u8> {
    val.clone().into_iter().collect()
    //~^ error: using clone on collection to own iterated items
}

fn partial_borrow(vals: (Vec<u8>,)) -> Vec<u8> {
    vals.0.clone().into_iter().collect()
    //~^ error: using clone on collection to own iterated items
}

fn generic<T: Clone>(val: Vec<T>) -> Vec<T> {
    val.clone().into_iter().collect()
    //~^ error: using clone on collection to own iterated items
}

fn use_mutable<T>(_: &mut T) {}

// Should not lint, as the replacement causes the mutable borrow to overlap
fn used_mutably<T: Clone>(mut vals: Vec<T>) {
    for val in vals.clone().into_iter() {
        use_mutable(&mut vals)
    }
}

// Should not lint, as the replacement causes the mutable borrow to overlap
fn used_mutably_chain<T: Clone>(mut vals: Vec<T>) {
    vals.clone().into_iter().for_each(|_| use_mutable(&mut vals));
}

// Should not lint, as the replacement causes the mutable borrow to overlap
fn used_mutably_partial_borrow<T: Clone>(mut vals: (Vec<T>,)) {
    vals.0.clone().into_iter().for_each(|_| use_mutable(&mut vals.0))
}

// Should not lint, as `Src` has no `iter` method to use.
fn too_generic<Src, Dst, T: Clone>(val: Src) -> Dst
where
    Src: IntoIterator<Item = T> + Clone,
    Dst: FromIterator<T>,
{
    val.clone().into_iter().collect()
}

fn main() {}

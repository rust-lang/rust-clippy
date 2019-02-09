#![allow(clippy::missing_copy_implementations)]
#![allow(clippy::missing_debug_implementations)]

#[derive(Clone)]
pub struct HashMap<V, S> {
    hash_builder: S,
    table: RawTable<V>,
}

#[derive(Clone)]
pub struct RawTable<V> {
    size: usize,
    val: V,
}

fn main() {}

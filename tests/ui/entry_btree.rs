#![warn(clippy::map_entry)]
#![allow(dead_code)]

use std::collections::BTreeMap;

fn foo() {}

fn btree_map<K: Eq + Ord + Copy, V: Copy>(m: &mut BTreeMap<K, V>, k: K, v: V) {
    // insert then do something, use if let
    if !m.contains_key(&k) {
    //~^ ERROR: usage of `contains_key` followed by `insert` on a `BTreeMap`
    //~| NOTE: `-D clippy::map-entry` implied by `-D warnings`
        m.insert(k, v);
        foo();
    }
}

fn main() {}

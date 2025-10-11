//@no-rustfix
#![warn(clippy::map_entry)]

use std::collections::HashMap;
use std::hash::Hash;

fn foo() {}

fn insert_if_absent0<K: Eq + Hash + Copy, V: Copy>(m: &mut HashMap<K, V>, k: K, v: V, v2: V) {
    if m.contains_key(&k) {
        //~^ map_entry
        if true { m.insert(k, v) } else { m.insert(k, v2) }
    } else {
        m.insert(k, v)
    };
}

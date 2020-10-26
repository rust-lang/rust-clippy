#![warn(clippy::entry_or_insert_with_default)]

use std::collections::{BTreeMap, HashMap};

fn btree(map: &mut BTreeMap<i32, i32>) {
    map.entry(42).or_insert_with(Default::default);
    map.entry(42).or_insert_with(i32::default);
    map.entry(42).or_default();
}

fn hash(map: &mut HashMap<i32, i32>) {
    map.entry(42).or_insert_with(Default::default);
    map.entry(42).or_insert_with(i32::default);
    map.entry(42).or_default();
}

struct InformalDefault;

impl InformalDefault {
    fn default() -> Self {
        InformalDefault
    }
}

fn not_default_btree(map: &mut BTreeMap<i32, InformalDefault>) {
    map.entry(42).or_insert_with(InformalDefault::default);
}

fn not_default_hash(map: &mut HashMap<i32, InformalDefault>) {
    map.entry(42).or_insert_with(InformalDefault::default);
}

#[derive(Default)]
struct NotAnEntry;

impl NotAnEntry {
    fn or_insert_with(self, f: impl Fn() -> Self) {}
}

fn main() {
    NotAnEntry.or_insert_with(Default::default);
    NotAnEntry.or_insert_with(NotAnEntry::default);
}

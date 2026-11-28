#![warn(clippy::hashmap_contains_key_then_get)]
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::ops::Neg;

fn main() {
    let m1: HashMap<i32, i32> = HashMap::new();
    let m2: HashMap<i32, i32> = HashMap::new();
    let bm: BTreeMap<i32, i32> = BTreeMap::new();
    let strct = SomeStruct::default();
    let map_wrapper = MapWrapper::default();

    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).unwrap();
        println!("val is equal to {val}");
    }

    // any unwrap-like method is linted
    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).expect("impossible!");
        println!("val is equal to {val}");
    }

    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).copied().unwrap_or_default();
        println!("val is equal to {val}");
    }

    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).unwrap_or(&199);
        println!("val is equal to {val}");
    }
    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).unwrap_or(&199);
        println!("val is equal to {val}");
    }

    // any methods in between the get and unwrap do not prevent the lint
    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).copied().map(i32::neg).filter(|it| *it > 64).unwrap();
        println!("val is equal to {val}");
    }

    //~v hashmap_contains_key_then_get
    if m1.contains_key(&1) {
        let val = m1.get(&1).cloned().unwrap();
        println!("val is equal to {val}");
    }

    // works if key is a field
    //~v hashmap_contains_key_then_get
    if m1.contains_key(&strct.a) {
        let val = m1.get(&strct.a).unwrap();
        println!("val is equal to {val}");
    }

    // works if map is a field
    //~v hashmap_contains_key_then_get
    if map_wrapper.m.contains_key(&1) {
        let val = map_wrapper.m.get(&1).unwrap();
        println!("val is equal to {val}");
    }

    //~v hashmap_contains_key_then_get
    if map_wrapper.m.contains_key(&strct.a) {
        let val = map_wrapper.m.get(&strct.a).unwrap();
        println!("val is equal to {val}");
    }

    // works properly when else clause is defined
    //~v hashmap_contains_key_then_get
    if map_wrapper.m.contains_key(&strct.a) {
        let val = map_wrapper.m.get(&strct.a).unwrap();
        println!("val is equal to {val}");
    } else {
        println!("no value!");
    }

    // works on BTreeMap as well
    //~v hashmap_contains_key_then_get
    if bm.contains_key(&1) {
        let val = bm.get(&1).unwrap();
        println!("val is equal to {val}");
    }

    // different recievers - no lint
    if m2.contains_key(&1) {
        let val = m1.get(&1).unwrap();
        println!("val is equal to {val}");
    }

    // different keys - no lint
    if m1.contains_key(&1) {
        let val = m1.get(&2).unwrap();
        println!("val is equal to {val}");
    }

    if m1.contains_key(&strct.a) {
        let val = m1.get(&strct.b).unwrap();
        println!("val is equal to {val}");
    }

    // suitable key, but possible side effects - unsafe to lint
    if m1.contains_key(&black_box(1)) {
        let val = m1.get(&black_box(1)).unwrap();
        println!("val is equal to {val}");
    }

    // possible side effects - no lint
    if black_box(&m1).contains_key(&1) {
        let val = black_box(&m1).get(&1).unwrap();
        println!("val is equal to {val}");
    }

    // value extracting expression is not the first in the block, no lint
    if m1.contains_key(&1) {
        println!("This is a terrible side effect!");
        let val = m1.get(&1).unwrap();
    }
}

#[derive(Default)]
struct MapWrapper {
    m: HashMap<i32, i32>,
}
#[derive(Default)]
struct SomeStruct {
    a: i32,
    b: i32,
}

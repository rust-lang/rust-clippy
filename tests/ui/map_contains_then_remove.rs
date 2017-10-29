#![feature(plugin)]
#![plugin(clippy)]

#![allow(unused)]
#![allow(collapsible_if)]

use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Display;

fn main() {
    contains();
    remove();
    contains_then_remove();
    remove_deeper();
    method_witness_then_remove();
    fn_witness_then_remove();
}

fn contains() {
    let mut map = HashMap::new();
    map.insert(1, "a");
    if map.contains_key(&1) {
        map.insert(2, "b");
    }
}

fn remove() {
    let mut map = HashMap::new();
    map.insert(1, "a");
    map.remove(&1);
}

fn contains_then_remove() {
    let mut map = HashMap::new();
    map.insert(1, "a");
    if map.contains_key(&1) {
        map.remove(&1); //should warn here
    }
}

fn remove_deeper() {
    let mut map = HashMap::new();
    map.insert(1, "a"); 
    if map.contains_key(&1) {
        if true {
            map.remove(&1); //should warn here
        }
    }
}

fn method_witness_then_remove() {
    let mut map = HashMap::new(); 
    map.insert(1, "a");
    if map.contains_key(&1) {
        let cap = map.capacity();
        map.remove(&1);       // should not warn here
    }
}

fn print_pairs<K: Eq + Hash + Display, V: Display>(map: &HashMap<K, V>) {
    for (key, val) in map.iter() {
        println!("key: {} val: {}", key, val);
    }
}

fn fn_witness_then_remove() {
    let mut map = HashMap::new();
    map.insert(1, "a");
    if map.contains_key(&1) {
        print_pairs(&map);
        map.remove(&1);    // should not warn here (failing)
    }
}


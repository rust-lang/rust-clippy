#![feature(plugin)]
#![plugin(clippy)]

#![allow(unused)]
#![allow(collapsible_if)]

use std::collections::HashMap;

fn main() {
    contains();
    remove();
    contains_then_remove();
    remove_deeper();
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


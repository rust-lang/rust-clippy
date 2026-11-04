#![warn(clippy::hash_bad)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;

fn main() {
    let _: HashMap<BadType, ()> = HashMap::default();
    //~^ hash_bad

    let _: HashSet<BadType> = HashSet::with_capacity(1);
    //~^ hash_bad

    let _: BTreeMap<BadType, ()> = Default::default();
    //~^ hash_bad

    let _: BTreeSet<BadType> = BTreeSet::new();
    //~^ hash_bad

    let _: HashSet<BadType> = HashSet::new();
    //~^ hash_bad

    let _: HashMap<BadType, ()> = HashMap::new();
    //~^ hash_bad

    let _: HashMap<EqOnlyType, ()> = HashMap::new();
    //~^ hash_bad

    // not an std Map/Set
    let _: Vec<BadType> = Vec::new();

    // GoodType implements requiered traits, no lint
    let _: HashMap<GoodType, ()> = HashMap::new();
    let _: BTreeMap<GoodType, ()> = BTreeMap::new();
    let _: HashSet<GoodType> = HashSet::new();
    let _: BTreeSet<GoodType> = BTreeSet::new();

    // BadType is used as a value - no lint
    let _: HashMap<GoodType, BadType> = HashMap::new();
}

struct BadType;

#[derive(Eq, PartialEq)]
struct EqOnlyType {
    x: i32,
}

#[derive(Eq, Hash, PartialEq)]
struct EqHashOnlyType {
    x: i32,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
struct GoodType {
    x: i32,
    y: i32,
}

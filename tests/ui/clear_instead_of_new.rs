#![warn(clippy::clear_instead_of_new)]
#![allow(unused)]
#![allow(clippy::vec_init_then_push)]
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

fn main() {
    // Should trigger lint - Vec
    let mut v = Vec::new();
    v.push(1);
    v = Vec::new();
    //~^ ERROR: assigning a new empty collection to `v`
    //~| NOTE: `-D clippy::clear-instead-of-new` implied by `-D warnings`
    //~| HELP: consider using `.clear()` instead
    //~| HELP: to override `-D warnings` add `#[allow(clippy::clear_instead_of_new)]`

    // Should trigger lint - HashMap
    let mut map = HashMap::new();
    map.insert(1, 2);
    map = HashMap::new();
    //~^ ERROR: assigning a new empty collection to `map`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - HashSet
    let mut set = HashSet::new();
    set.insert(1);
    set = HashSet::new();
    //~^ ERROR: assigning a new empty collection to `set`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - VecDeque
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque = VecDeque::new();
    //~^ ERROR: assigning a new empty collection to `deque`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BTreeMap
    let mut btree_map = BTreeMap::new();
    btree_map.insert(1, 2);
    btree_map = BTreeMap::new();
    //~^ ERROR: assigning a new empty collection to `btree_map`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BTreeSet
    let mut btree_set = BTreeSet::new();
    btree_set.insert(1);
    btree_set = BTreeSet::new();
    //~^ ERROR: assigning a new empty collection to `btree_set`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BinaryHeap
    let mut heap = BinaryHeap::new();
    heap.push(1);
    heap = BinaryHeap::new();
    //~^ ERROR: assigning a new empty collection to `heap`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list = LinkedList::new();
    //~^ ERROR: assigning a new empty collection to `list`
    //~| HELP: consider using `.clear()` instead
}

fn should_not_trigger() {
    // Different variable - OK
    let mut v1: Vec<i32> = Vec::new();
    let mut v2: Vec<i32> = Vec::new();

    // First assignment to uninitialized - OK (allow so rustfix doesn't suggest .clear() on
    // uninitialized)
    let mut v3: Vec<i32>;
    #[allow(clippy::clear_instead_of_new)]
    {
        v3 = Vec::new();
    }

    // With type parameters
    let mut v4: Vec<i32> = Vec::new();
    v4.push(1);
    v4 = Vec::new(); // Should still trigger
    //~^ ERROR: assigning a new empty collection to `v4`
    //~| HELP: consider using `.clear()` instead

    // Collection with capacity - should not trigger (different from new())
    let mut v5: Vec<i32> = Vec::with_capacity(10);
    v5 = Vec::new(); // OK - with_capacity is different
    //~^ ERROR: assigning a new empty collection to `v5`
    //~| HELP: consider using `.clear()` instead
}

fn edge_cases() {
    // Assignment in an expression context
    let mut v = Vec::new();
    v.push(1);
    let _ = {
        v = Vec::new();
        //~^ ERROR: assigning a new empty collection to `v`
        //~| HELP: consider using `.clear()` instead
        42
    };

    // Multiple assignments
    let mut v2: Vec<i32> = Vec::new();
    v2 = Vec::new();
    //~^ ERROR: assigning a new empty collection to `v2`
    //~| HELP: consider using `.clear()` instead
    v2 = Vec::new();
    //~^ ERROR: assigning a new empty collection to `v2`
    //~| HELP: consider using `.clear()` instead
}

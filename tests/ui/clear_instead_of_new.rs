#![warn(clippy::clear_instead_of_new)]
#![allow(clippy::vec_init_then_push)]
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

fn use_vec(_v: &Vec<i32>) {}
fn use_map(_m: &HashMap<i32, i32>) {}
fn use_set(_s: &HashSet<i32>) {}
fn use_deque(_d: &VecDeque<i32>) {}
fn use_btree_map(_m: &BTreeMap<i32, i32>) {}
fn use_btree_set(_s: &BTreeSet<i32>) {}
fn use_heap(_h: &BinaryHeap<i32>) {}
fn use_list(_l: &LinkedList<i32>) {}

fn main() {
    // Should trigger lint - Vec
    let mut v = Vec::new();
    v.push(1);
    use_vec(&v);
    v = Vec::new();
    //~^ ERROR: assigning a new empty collection
    //~| NOTE: `-D clippy::clear-instead-of-new` implied by `-D warnings`
    //~| HELP: to override `-D warnings` add `#[allow(clippy::clear_instead_of_new)]`
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - HashMap
    let mut map = HashMap::new();
    map.insert(1, 2);
    use_map(&map);
    map = HashMap::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - HashSet
    let mut set = HashSet::new();
    set.insert(1);
    use_set(&set);
    set = HashSet::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - VecDeque
    let mut deque = VecDeque::new();
    deque.push_back(1);
    use_deque(&deque);
    deque = VecDeque::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BTreeMap
    let mut btree_map = BTreeMap::new();
    btree_map.insert(1, 2);
    use_btree_map(&btree_map);
    btree_map = BTreeMap::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BTreeSet
    let mut btree_set = BTreeSet::new();
    btree_set.insert(1);
    use_btree_set(&btree_set);
    btree_set = BTreeSet::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - BinaryHeap
    let mut heap = BinaryHeap::new();
    heap.push(1);
    use_heap(&heap);
    heap = BinaryHeap::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    // Should trigger lint - LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    use_list(&list);
    list = LinkedList::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead
}

fn test_dereference() {
    // Should trigger lint with dereference pattern
    let mut v = Box::new(Vec::new());
    v.push(1);
    use_vec(&v);
    *v = Vec::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead
}

fn should_not_trigger() {
    // Different variable - OK
    let mut v1: Vec<i32> = Vec::new();
    let mut v2: Vec<i32> = Vec::new();

    // First assignment to uninitialized - this is a known false positive
    // TODO: improve lint to detect uninitialized variables
    let mut v3: Vec<i32>;
    #[allow(clippy::clear_instead_of_new)]
    {
        v3 = Vec::new();
    }

    // With type parameters - should trigger
    let mut v4: Vec<i32> = Vec::new();
    v4.push(1);
    use_vec(&v4);
    v4 = Vec::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead
}

fn edge_cases() {
    // Assignment in an expression context
    let mut v = Vec::new();
    v.push(1);
    use_vec(&v);
    let _ = {
        v = Vec::new();
        //~^ ERROR: assigning a new empty collection
        //~| HELP: consider using `.clear()` instead
        42
    };

    // Multiple assignments
    let mut v2: Vec<i32> = Vec::new();
    v2 = Vec::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead

    v2.push(1);
    use_vec(&v2);
    v2 = Vec::new();
    //~^ ERROR: assigning a new empty collection
    //~| HELP: consider using `.clear()` instead
}

// Test that lint doesn't trigger in macros
macro_rules! assign_new {
    ($v:ident) => {
        $v = Vec::new()
    };
}

fn macro_test() {
    let mut v = Vec::new();
    v.push(1);
    use_vec(&v);
    assign_new!(v); // Should NOT trigger - in macro expansion
}

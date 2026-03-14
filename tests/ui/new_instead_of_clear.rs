#![warn(clippy::new_instead_of_clear)]
#![expect(clippy::vec_init_then_push)]
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

fn use_vec(_: &Vec<i32>) {}
fn use_map(_: &HashMap<i32, i32>) {}
fn use_set(_: &HashSet<i32>) {}
fn use_deque(_: &VecDeque<i32>) {}
fn use_btree_map(_: &BTreeMap<i32, i32>) {}
fn use_btree_set(_: &BTreeSet<i32>) {}
fn use_heap(_: &BinaryHeap<i32>) {}
fn use_list(_: &LinkedList<i32>) {}

fn main() {
    let mut v = Vec::new();
    v.push(1);
    use_vec(&v);
    v = Vec::new();
    //~^ new_instead_of_clear

    let mut map: HashMap<i32, i32> = HashMap::new();
    map.insert(1, 2);
    use_map(&map);
    map = HashMap::new();
    //~^ new_instead_of_clear

    let mut set: HashSet<i32> = HashSet::new();
    set.insert(1);
    use_set(&set);
    set = HashSet::new();
    //~^ new_instead_of_clear

    let mut deque: VecDeque<i32> = VecDeque::new();
    deque.push_back(1);
    use_deque(&deque);
    deque = VecDeque::new();
    //~^ new_instead_of_clear

    let mut btree_map: BTreeMap<i32, i32> = BTreeMap::new();
    btree_map.insert(1, 2);
    use_btree_map(&btree_map);
    btree_map = BTreeMap::new();
    //~^ new_instead_of_clear

    let mut btree_set: BTreeSet<i32> = BTreeSet::new();
    btree_set.insert(1);
    use_btree_set(&btree_set);
    btree_set = BTreeSet::new();
    //~^ new_instead_of_clear

    let mut heap: BinaryHeap<i32> = BinaryHeap::new();
    heap.push(1);
    use_heap(&heap);
    heap = BinaryHeap::new();
    //~^ new_instead_of_clear

    let mut list: LinkedList<i32> = LinkedList::new();
    list.push_back(1);
    use_list(&list);
    list = LinkedList::new();
    //~^ new_instead_of_clear

    let mut v2 = Vec::new();
    v2.push(1);
    use_vec(&v2);
    v2 = vec![];
    //~^ new_instead_of_clear
}

fn deref_target() {
    let mut v = Box::new(Vec::<i32>::new());
    v.push(1);
    use_vec(&v);
    *v = Vec::new();
    //~^ new_instead_of_clear
}

fn block_expr() {
    let mut v = Vec::new();
    v.push(1);
    use_vec(&v);
    let _ = {
        v = Vec::new();
        //~^ new_instead_of_clear
        42
    };
}

fn no_lint() {
    // turbofish changes type inference, replacing with .clear() would be wrong
    let mut v: Vec<i32> = Vec::new();
    v.push(1);
    use_vec(&v);
    v = Vec::<i32>::new();

    // first assignment to an uninitialized binding, .clear() won't compile
    let mut u: Vec<u8>;
    u = Vec::new();
    u.push(1u8);

    // inside a macro expansion
    macro_rules! reset {
        ($x:ident) => {
            $x = Vec::new()
        };
    }
    let mut w = Vec::new();
    w.push(1);
    use_vec(&w);
    reset!(w);
}

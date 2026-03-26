#![allow(unused)]
#![warn(clippy::clear_then_shrink)]

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;

fn side_effect_vec() -> Vec<i32> {
    vec![1, 2, 3]
}

fn main() {
    let mut v = vec![1, 2, 3];
    v.clear();
    //~^ clear_then_shrink
    v.shrink_to_fit();

    let v = &mut vec![1, 2, 3];
    v.clear();
    //~^ clear_then_shrink
    v.shrink_to_fit();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.clear();
    //~^ clear_then_shrink
    deque.shrink_to_fit();

    let mut s = String::from("abc");
    s.clear();
    //~^ clear_then_shrink
    s.shrink_to_fit();

    let mut path = PathBuf::from("tmp");
    path.clear();
    //~^ clear_then_shrink
    path.shrink_to_fit();

    let mut os = OsString::from("tmp");
    os.clear();
    //~^ clear_then_shrink
    os.shrink_to_fit();

    let mut map = HashMap::from([(1, 2)]);
    map.clear();
    //~^ clear_then_shrink
    map.shrink_to_fit();

    let mut set = HashSet::from([1, 2, 3]);
    set.clear();
    //~^ clear_then_shrink
    set.shrink_to_fit();

    let mut heap = BinaryHeap::from([1, 2, 3]);
    heap.clear();
    //~^ clear_then_shrink
    heap.shrink_to_fit();

    let mut no_lint = vec![1, 2, 3];
    no_lint.clear();
    println!("not consecutive");
    no_lint.shrink_to_fit();

    let mut lhs = vec![1, 2, 3];
    let mut rhs = vec![1, 2, 3];
    lhs.clear();
    rhs.shrink_to_fit();

    side_effect_vec().clear();
    side_effect_vec().shrink_to_fit();
}

#[clippy::msrv = "1.39.0"]
fn msrv_1_39() {
    let mut v = vec![1, 2, 3];
    v.clear();
    v.shrink_to_fit();
}

#[clippy::msrv = "1.40.0"]
fn msrv_1_40() {
    let mut v = vec![1, 2, 3];
    v.clear();
    //~^ clear_then_shrink
    v.shrink_to_fit();
}

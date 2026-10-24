#![warn(clippy::iter_with_drain)]
#![expect(clippy::drain_collect)]

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

fn full() {
    let mut a = vec!["aaa".to_string(), "bbb".to_string()];
    let mut a: BinaryHeap<_> = a.drain(..).collect();
    //~^ iter_with_drain
    let mut a: HashSet<_> = a.drain().collect();
    let mut a: VecDeque<_> = a.drain().collect();
    let mut a: Vec<_> = a.drain(..).collect();
    //~^ iter_with_drain
    let mut a: HashMap<_, _> = a.drain(..).map(|x| (x.clone(), x)).collect();
    //~^ iter_with_drain
    let _: Vec<(String, String)> = a.drain().collect();
}

fn closed() {
    let mut a = vec!["aaa".to_string(), "bbb".to_string()];
    let mut a: BinaryHeap<_> = a.drain(0..).collect();
    //~^ iter_with_drain
    let mut a: HashSet<_> = a.drain().collect();
    let mut a: VecDeque<_> = a.drain().collect();
    let mut a: Vec<_> = a.drain(..a.len()).collect();
    //~^ iter_with_drain
    let mut a: HashMap<_, _> = a.drain(0..a.len()).map(|x| (x.clone(), x)).collect();
    //~^ iter_with_drain
    let _: Vec<(String, String)> = a.drain().collect();
}

fn should_not_help() {
    let mut a = vec!["aaa".to_string(), "bbb".to_string()];
    let mut a: BinaryHeap<_> = a.drain(1..).collect();
    let mut a: HashSet<_> = a.drain().collect();
    let mut a: VecDeque<_> = a.drain().collect();
    let mut a: Vec<_> = a.drain(..a.len() - 1).collect();
    let mut a: HashMap<_, _> = a.drain(1..a.len() - 1).map(|x| (x.clone(), x)).collect();
    let _: Vec<(String, String)> = a.drain().collect();

    let mut b = vec!["aaa".to_string(), "bbb".to_string()];
    let _: Vec<_> = b.drain(0..a.len()).collect();
}

fn _closed_range(mut x: Vec<String>) {
    let _: Vec<String> = x.drain(0..=x.len()).collect();
}

fn _with_mut(x: &mut Vec<String>, y: &mut VecDeque<String>) {
    let _: Vec<String> = x.drain(..).collect();
    let _: Vec<String> = y.drain(..).collect();
}

#[derive(Default)]
struct Bomb {
    fire: Vec<u8>,
}

fn should_not_help_0(bomb: &mut Bomb) {
    let _: Vec<u8> = bomb.fire.drain(..).collect();
}

// https://github.com/rust-lang/rust-clippy/issues/15119
// `into_iter()` would move `current`, which the loop condition reads again.
fn issue_15119() {
    let mut current = vec![];
    while !current.is_empty() {
        current.drain(..).for_each(|_: ()| {});
    }
}

fn should_not_help_reused_by_loop() {
    let mut a = vec![1, 2, 3];
    for _ in 0..3 {
        // `a` is drained again on the next iteration even though it isn't mentioned again.
        let _: Vec<_> = a.drain(..).collect();
    }
}

fn should_not_help_used_after() {
    let mut a = vec![1, 2, 3];
    let _: Vec<_> = a.drain(..).collect();
    a.push(4);
}

fn should_not_help_swapped_layers() {
    let mut curr = vec![1];
    let mut next = vec![];
    while !curr.is_empty() {
        curr.drain(..).for_each(|x| {
            if x < 4 {
                next.push(x + 1);
            }
        });
        std::mem::swap(&mut curr, &mut next);
    }
}

fn should_not_help_fn_mut_closure() {
    let mut a = vec![1, 2, 3];
    let mut f = || {
        let _: Vec<_> = a.drain(..).collect();
    };
    f();
    f();
}

fn should_help_declared_inside_loop() {
    for _ in 0..3 {
        let mut a = vec![1, 2, 3];
        let _: Vec<_> = a.drain(..).collect();
        //~^ iter_with_drain
    }
}

fn should_help_last_use() {
    let mut a = vec![1, 2, 3];
    a.push(4);
    let _: Vec<_> = a.drain(..).collect();
    //~^ iter_with_drain
}

// `into_iter()` can't move out of a deref or an index.
fn should_not_help_place_receivers(x: &mut Vec<String>, y: &mut [Vec<String>]) {
    let _: Vec<String> = (*x).drain(..).collect();
    let _: Vec<String> = y[0].drain(..).collect();
}

fn main() {}

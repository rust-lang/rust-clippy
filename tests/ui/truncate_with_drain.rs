#![allow(unused)]
#![warn(clippy::truncate_with_drain)]
use std::collections::VecDeque;
fn vec_range() {
    // Do not lint because iterator is assigned
    let mut v = vec![1, 2, 3];
    let iter = v.drain(1..v.len());

    // Do not lint because iterator is used
    let mut v = vec![1, 2, 3];
    v.drain(1..v.len()).next();

    // Do lint
    let mut v = vec![1, 2, 3];
    v.drain(1..v.len());
    //~^ ERROR: `drain` used to truncate a `Vec`

    // Do lint
    let x = 1;
    let mut v = vec![1, 2, 3];
    v.drain(x..v.len());
    //~^ ERROR: `drain` used to truncate a `Vec`
}

fn vec_range_from() {
    // Do not lint because iterator is assigned
    let mut v = vec![1, 2, 3];
    let iter = v.drain(1..);

    // Do not lint because iterator is used
    let mut v = vec![1, 2, 3];
    v.drain(1..).next();

    // Do lint
    let mut v = vec![1, 2, 3];
    v.drain(1..);
    //~^ ERROR: `drain` used to truncate a `Vec`

    // Do lint
    let x = 1;
    let mut v = vec![1, 2, 3];
    v.drain(x..);
    //~^ ERROR: `drain` used to truncate a `Vec`
}

fn vec_partial_drains() {
    // Do not lint any of these because the ranges are not open-ended
    let mut v = vec![1, 2, 3];
    v.drain(..v.len() - 1);

    let mut v = vec![1, 2, 3];
    v.drain(..v.len() - 1).min();

    let mut v = vec![1, 2, 3];
    v.drain(0..v.len() - 1);

    let mut v = vec![1, 2, 3];
    v.drain(0..v.len() - 1).min();

    let mut v = vec![1, 2, 3];
    v.drain(usize::MIN..v.len() - 1);

    let mut v = vec![1, 2, 3];
    v.drain(usize::MIN..v.len() - 1).min();

    let mut v = vec![1, 2, 3];
    v.drain(..);

    let mut v = vec![1, 2, 3];
    v.drain(..).min();

    let mut v = vec![1, 2, 3];
    v.drain(0..v.len());

    let mut v = vec![1, 2, 3];
    v.drain(0..v.len()).min();

    let mut v = vec![1, 2, 3];
    v.drain(0..);

    let mut v = vec![1, 2, 3];
    v.drain(0..).min();

    let mut v = vec![1, 2, 3];
    v.drain(usize::MIN..);

    let mut v = vec![1, 2, 3];
    v.drain(usize::MIN..).min();

    let mut v = vec![1, 2, 3];
    v.drain(1..v.len() - 1);

    let mut v = vec![1, 2, 3];
    let w: Vec<i8> = v.drain(1..v.len() - 1).collect();
}
fn vec_deque_range() {
    // Do not lint because iterator is assigned
    let mut deque = VecDeque::from([1, 2, 3]);
    let iter = deque.drain(1..deque.len());

    // Do not lint because iterator is used
    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(1..deque.len()).next();

    // Do lint
    let mut v = VecDeque::from([1, 2, 3]);
    v.drain(1..v.len());
    //~^ ERROR: `drain` used to truncate a `VecDeque`

    // Do lint
    let x = 1;
    let mut v = VecDeque::from([1, 2, 3]);
    v.drain(x..v.len());
    //~^ ERROR: `drain` used to truncate a `VecDeque`
}

fn vec_deque_range_from() {
    // Do not lint because iterator is assigned
    let mut deque = VecDeque::from([1, 2, 3]);
    let iter = deque.drain(1..);

    // Do not lint because iterator is used
    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(1..).next();

    // Do lint
    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(1..);
    //~^ ERROR: `drain` used to truncate a `VecDeque`

    // Do lint
    let x = 1;
    let mut v = VecDeque::from([1, 2, 3]);
    v.drain(x..);
    //~^ ERROR: `drain` used to truncate a `VecDeque`
}

fn vec_deque_partial_drains() {
    // Do not lint any of these because the ranges are not open-ended
    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(..deque.len() - 1);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(..deque.len() - 1).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..deque.len() - 1);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..deque.len() - 1).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(usize::MIN..deque.len() - 1);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(usize::MIN..deque.len() - 1).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(..);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(..).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..deque.len());

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..deque.len()).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(0..).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(usize::MIN..);

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(usize::MIN..).min();

    let mut deque = VecDeque::from([1, 2, 3]);
    deque.drain(1..deque.len() - 1);

    let mut deque = VecDeque::from([1, 2, 3]);
    let w: VecDeque<i8> = deque.drain(1..deque.len() - 1).collect();
}
fn string_range() {
    // Do not lint because iterator is assigned
    let mut s = String::from("Hello, world!");
    let iter = s.drain(1..s.len());

    // Do not lint because iterator is used
    let mut s = String::from("Hello, world!");
    s.drain(1..s.len()).next();

    // Do lint
    let mut s = String::from("Hello, world!");
    s.drain(1..s.len());
    //~^ ERROR: `drain` used to truncate a `String`

    // Do lint
    let x = 1;
    let mut s = String::from("Hello, world!");
    s.drain(x..s.len());
    //~^ ERROR: `drain` used to truncate a `String`
}

fn string_range_from() {
    // Do not lint because iterator is assigned
    let mut s = String::from("Hello, world!");
    let iter = s.drain(1..);

    // Do not lint because iterator is used
    let mut s = String::from("Hello, world!");
    s.drain(1..).next();

    // Do lint
    let mut s = String::from("Hello, world!");
    s.drain(1..);
    //~^ ERROR: `drain` used to truncate a `String`

    // Do lint
    let x = 1;
    let mut s = String::from("Hello, world!");
    s.drain(x..);
    //~^ ERROR: `drain` used to truncate a `String`
}

fn string_partial_drains() {
    // Do not lint any of these because the ranges are not open-ended
    let mut s = String::from("Hello, world!");
    s.drain(..s.len() - 1);

    let mut s = String::from("Hello, world!");
    s.drain(..s.len() - 1).min();

    let mut s = String::from("Hello, world!");
    s.drain(0..s.len() - 1);

    let mut s = String::from("Hello, world!");
    s.drain(0..s.len() - 1).min();

    let mut s = String::from("Hello, world!");
    s.drain(usize::MIN..s.len() - 1);

    let mut s = String::from("Hello, world!");
    s.drain(usize::MIN..s.len() - 1).min();

    let mut s = String::from("Hello, world!");
    s.drain(..);

    let mut s = String::from("Hello, world!");
    s.drain(..).min();

    let mut s = String::from("Hello, world!");
    s.drain(0..s.len());

    let mut s = String::from("Hello, world!");
    s.drain(0..s.len()).min();

    let mut s = String::from("Hello, world!");
    s.drain(0..);

    let mut s = String::from("Hello, world!");
    s.drain(0..).min();

    let mut s = String::from("Hello, world!");
    s.drain(usize::MIN..);

    let mut s = String::from("Hello, world!");
    s.drain(usize::MIN..).min();

    let mut s = String::from("Hello, world!");
    s.drain(1..s.len() - 1);

    let mut s = String::from("Hello, world!");
    let w: String = s.drain(1..s.len() - 1).collect();
}
fn main() {}

#![warn(clippy::unnecessary_reserve)]
#![feature(custom_inner_attributes)]

use std::collections::HashMap;
use std::collections::VecDeque;

fn main() {
    vec_reserve();
    vec_deque_reserve();
    hash_map_reserve();
    msrv_1_62();
}

fn vec_reserve() {
    let mut vec: Vec<usize> = vec![];
    let array: &[usize] = &[1, 2];

    // do lint
    vec.reserve(1);
    vec.extend([1]);

    // do lint
    vec.reserve(array.len());
    vec.extend(array);

    // do lint
    {
        vec.reserve(1);
        vec.extend([1])
    };

    // do not lint
    vec.reserve(array.len());
    vec.push(1);
    vec.extend(array);

    // do not lint
    let mut other_vec: Vec<usize> = vec![];
    other_vec.reserve(1);
    vec.extend([1])
}

fn vec_deque_reserve() {
    let mut vec_deque: VecDeque<usize> = [1].into();
    let array: &[usize] = &[1, 2];

    // do lint
    vec_deque.reserve(1);
    vec_deque.extend([1]);

    // do lint
    vec_deque.reserve(array.len());
    vec_deque.extend(array);

    // do lint
    {
        vec_deque.reserve(1);
        vec_deque.extend([1])
    };

    // do not lint
    vec_deque.reserve(array.len() + 1);
    vec_deque.push_back(1);
    vec_deque.extend(array);

    // do not lint
    let mut other_vec_deque: VecDeque<usize> = [1].into();
    other_vec_deque.reserve(1);
    vec_deque.extend([1])
}

fn hash_map_reserve() {
    let mut map: HashMap<usize, usize> = HashMap::new();
    let mut other_map: HashMap<usize, usize> = HashMap::new();
    // do not lint
    map.reserve(other_map.len());
    map.extend(other_map);
}

fn msrv_1_62() {
    #![clippy::msrv = "1.61"]
    let mut vec: Vec<usize> = vec![];
    let array: &[usize] = &[1, 2];

    // do not lint
    vec.reserve(1);
    vec.extend([1]);

    let mut vec_deque: VecDeque<usize> = [1].into();
    let array: &[usize] = &[1, 2];

    // do not lint
    vec_deque.reserve(1);
    vec_deque.extend([1]);
}

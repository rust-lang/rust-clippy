#![warn(clippy::swap_lock_guards)]

use std::sync::RwLock;

fn main() {
    let left = RwLock::new(1);
    let right = RwLock::new(2);
    let mut left_guard = left.read().unwrap();
    let mut right_guard = right.read().unwrap();

    std::mem::swap(&mut left_guard, &mut right_guard);
    //~^ swap_lock_guards
}

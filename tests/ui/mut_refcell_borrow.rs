#![warn(clippy::mut_mutex_lock)]

use std::cell::RefCell;
use std::rc::Rc;

pub fn replace(x: &mut RefCell<i32>) {
    x.replace(0);
}

pub fn replace_with(x: &mut RefCell<i32>) {
    x.replace_with(|&mut old| old + 1);
}

pub fn borrow(x: &mut RefCell<i32>) {
    let _: i32 = *x.borrow();
}

pub fn try_borrow(x: &mut RefCell<i32>) {
    let _: i32 = *x.try_borrow().unwrap();
}

pub fn borrow_mut(x: &mut RefCell<i32>) {
    *x.borrow_mut() += 1;
}

pub fn try_borrow_mut(x: &mut RefCell<i32>) {
    *x.try_borrow_mut().unwrap() += 1;
}

pub fn take(x: &mut RefCell<i32>) {
    let _: i32 = x.take();
}

// must not lint
pub fn deref_refcell(x: Rc<RefCell<i32>>) {
    *x.borrow_mut() += 1;
}

// must not lint
pub fn mut_deref_refcell(x: &mut Rc<RefCell<i32>>) {
    *x.borrow_mut() += 1;
}

fn main() {}

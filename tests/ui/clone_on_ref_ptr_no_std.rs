#![warn(clippy::clone_on_ref_ptr)]
#![no_std]

extern crate alloc;
use alloc::rc::{Rc, Weak as RcWeak};
use alloc::sync::{Arc, Weak as ArcWeak};

fn clone_rc(x: &Rc<i32>) -> Rc<i32> {
    x.clone()
    //~^ clone_on_ref_ptr
}

fn clone_rc_owned(x: Rc<i32>) -> Rc<i32> {
    x.clone()
    //~^ clone_on_ref_ptr
}

fn clone_weak(x: &RcWeak<i32>) -> RcWeak<i32> {
    x.clone()
    //~^ clone_on_ref_ptr
}

fn clone_arc(x: &Arc<i32>) -> Arc<i32> {
    x.clone()
    //~^ clone_on_ref_ptr
}

fn clone_arc_weak(x: &ArcWeak<i32>) -> ArcWeak<i32> {
    x.clone()
    //~^ clone_on_ref_ptr
}

// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(unsize, coerce_unsized)]
#![deny(clippy::wrong_any_coerce)]
#![deny(bare_trait_objects)]

use std::any::Any;
use std::cell::RefCell;
use std::fmt::Debug;
use std::iter::Iterator;
use std::marker::{Send, Unsize};
use std::ops::CoerceUnsized;
use std::ops::Deref;
use std::rc::Rc;

struct Foo;

struct Unsizeable<T: ?Sized, U: ?Sized, V: ?Sized> {
    box_v: Box<V>,
    rc_t: Rc<T>,
    u: U,
}

fn main() {
    let mut box_any: Box<dyn Any + Send> = Box::new(Foo);
    let _: *mut dyn Any = &mut box_any; // LINT
    let _: *mut dyn Any = &mut *box_any; // ok

    let rc_rc_any: Rc<Rc<dyn Any>> = Rc::new(Rc::new(Foo));
    let _: &dyn Any = &rc_rc_any; // LINT
    let _: &dyn Any = &*rc_rc_any; // LINT
    let _: &dyn Any = &**rc_rc_any; // ok
    let _: &Rc<dyn Any> = &*rc_rc_any; // ok

    let refcell_box_any: RefCell<Box<dyn Any>> = RefCell::new(Box::new(Foo));
    let _: &RefCell<dyn Any> = &refcell_box_any; // LINT

    let rc_unsizable_rc_any: Rc<Unsizeable<i32, Rc<dyn Any>, i32>> = Rc::new(Unsizeable {
        box_v: Box::new(0),
        rc_t: Rc::new(0),
        u: Rc::new(Foo),
    });
    let _: Rc<Unsizeable<i32, dyn Any, i32>> = rc_unsizable_rc_any.clone(); // LINT
    let _: &Unsizeable<i32, dyn Any, i32> = &*rc_unsizable_rc_any; // LINT
    let _: &Rc<Unsizeable<i32, Rc<dyn Any>, i32>> = &rc_unsizable_rc_any; // ok
    let _: &Unsizeable<i32, Rc<dyn Any>, i32> = &*rc_unsizable_rc_any; // ok

    let ref_any: &dyn Any = &Foo;
    let _: &dyn Any = &ref_any; // LINT
    let _: &dyn Any = &*ref_any; // ok

    let ref_refcell_any: &'static RefCell<dyn Any> = Box::leak(Box::new(RefCell::new(Foo)));
    let _: &dyn Any = &ref_refcell_any.borrow(); // LINT
    let _: &dyn Any = &*ref_refcell_any.borrow(); // ok
}

fn very_generic<T, U>(t: &'static T)
where
    T: Deref<Target = U> + 'static,
    U: Deref<Target = dyn Any + Send> + 'static,
{
    let _: &dyn Any = t; // LINT
    let _: &dyn Any = &t; // LINT
    let _: &dyn Any = &*t; // LINT
    let _: &dyn Any = &**t; // LINT
    let _: &dyn Any = &***t; // ok
}

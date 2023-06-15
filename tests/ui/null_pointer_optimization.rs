#![allow(clippy::boxed_local, unused)]
#![warn(clippy::null_pointer_optimization)]
#![no_main]

use std::ptr::NonNull;

type A<T> = Box<Option<T>>;

fn a<T>(a: Box<Option<T>>) {}

// lint at `type A<T>`, but not here.
fn a_ty_alias<T>(a: A<T>) {}

fn b(b: String) {}

fn c<T>(c: NonNull<Option<T>>) {}

fn g<T>(e: Option<Box<Option<T>>>) {}

struct H<T>(Box<Option<T>>);

enum I<T> {
    J(Box<Option<T>>),
}

// do not lint

struct D<T>(Option<T>);

fn d<T>(d: D<T>) {}

fn e<T>(e: Option<NonNull<T>>) {}

fn f<T>(e: Option<Box<T>>) {}

#![warn(clippy::refcell_cell)]

use std::cell::{Cell, RefCell};
use std::hint::black_box;

struct NameFieldStruct<T: Copy, U> {
    ng: RefCell<T>, //~ refcell_cell
    ok: RefCell<U>,
}

struct TupleStruct<T: Copy, U>(
    RefCell<T>, //~ refcell_cell
    RefCell<U>,
);

enum Enum<T: Copy, U> {
    NG(RefCell<T>), //~ refcell_cell
    OK(RefCell<U>),
    Named {
        ng: RefCell<T>, //~ refcell_cell
        ok: RefCell<U>,
    },
}

trait Trait<T: Copy, U> {
    fn func(
        ng: RefCell<T>, //~ refcell_cell
        ok: RefCell<U>,
    ) -> (
        RefCell<T>, //~ refcell_cell
        RefCell<U>,
    );
}

struct Dummy;

impl<T: Copy, U> Trait<T, U> for Dummy {
    fn func(ng: RefCell<T>, ok: RefCell<U>) -> (RefCell<T>, RefCell<U>) {
        todo!()
    }
}

fn func<T: Copy, U>(
    ng: RefCell<T>, //~ refcell_cell
    ok: RefCell<U>,
) -> (
    RefCell<T>, //~ refcell_cell
    RefCell<U>,
) {
    todo!()
}

impl Dummy {
    type NG = RefCell<i32>; //~ refcell_cell
    type OK = RefCell<String>;

    fn func<T: Copy, U>(
        ng: RefCell<T>, //~ refcell_cell
        ok: RefCell<U>,
    ) -> (
        RefCell<T>, //~ refcell_cell
        RefCell<U>,
    ) {
        todo!()
    }
}

fn main() {
    let _ = RefCell::new(1); //~ refcell_cell
    let _ = RefCell::new("Rust".to_string());

    // False negative: This lint does not check trait bounds on call site.
    let _ = black_box(RefCell::new(1));
}

//@no-rustfix
#![warn(clippy::refcell_cell)]

use std::cell::{Cell, RefCell};
use std::hint::black_box;

struct NameFieldStruct<'a, T: Copy, U> {
    ng: RefCell<T>, //~ refcell_cell
    ok: RefCell<U>,
    ng_ref: &'a RefCell<T>, //~ refcell_cell
    ok_ref: &'a RefCell<U>,
    large_copy: RefCell<[i32; 16]>,
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

type AliasNG = RefCell<i32>; //~ refcell_cell

type AliasOK<T> = RefCell<T>;

trait Trait<T: Copy, U> {
    // associated type defaults are unstable
    type NG;
    type OK;

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
    type NG = RefCell<T>; //~ refcell_cell
    type OK = RefCell<U>;

    fn func(ng: RefCell<T>, ok: RefCell<U>) -> (RefCell<T>, RefCell<U>) {
        todo!()
    }
}

fn func<'a, T: Copy, U>(
    ng: RefCell<T>, //~ refcell_cell
    ok: RefCell<U>,
    ng_ref: &'a RefCell<T>, //~ refcell_cell
    ok_ref: &'a RefCell<U>,
) -> (
    RefCell<T>, //~ refcell_cell
    RefCell<U>,
    &'a RefCell<T>, // should be linted on definition site
    &'a RefCell<U>,
) {
    todo!()
}

impl Dummy {
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
    let _ = RefCell::<i32>::new(1); //~ refcell_cell
    let _ = RefCell::<_>::new(1); //~ refcell_cell
    // non-`Copy` type
    let _ = RefCell::new(vec![1]);
    // large `Copy` type
    let _ = RefCell::new([1; 1024]);

    let _ = RefCell::from(1); //~ refcell_cell
    let _ = RefCell::<i32>::from(1); //~ refcell_cell

    let _: RefCell<i32> = Default::default(); //~ refcell_cell
    let _ = <RefCell<i32> as Default>::default(); //~ refcell_cell

    // False negative: This lint does not check trait bounds on call site.
    let _ = black_box(RefCell::new(1));

    // Macro expansion
    macro_rules! black_box {
        ( $e:expr ) => {
            black_box($e)
        };
    }
    let _ = RefCell::new(black_box!(1));
    let _ = RefCell::from(black_box!(1));
    let _ = black_box!(RefCell::new(1));

    macro_rules! ignore_me {
        () => {
            let _ = RefCell::new(1);
        };
    }
    ignore_me!();
}

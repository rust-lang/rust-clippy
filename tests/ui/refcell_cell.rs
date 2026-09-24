//@no-rustfix
#![warn(clippy::refcell_cell)]

use std::cell::{Cell, RefCell};
use std::hint::black_box;

struct NameFieldStruct<'a, T: Copy, U> {
    ng: RefCell<T>, //~ refcell_cell
    ok: RefCell<U>,
    ng_ref: &'a RefCell<T>, //~ refcell_cell
    ok_ref: &'a RefCell<U>,
    large_copy: RefCell<[i32; 1024]>,
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

    fn ng(ng: RefCell<T>); //~ refcell_cell
    fn ok(ok: RefCell<U>);
    fn ng_ref(ng: &RefCell<T>); //~ refcell_cell
    fn ok_ref(ok: &RefCell<U>);
    fn ng_ret() -> RefCell<T>; //~ refcell_cell
    fn ok_ret() -> RefCell<U>;
    fn ok_ret_ref<'a>() -> &'a RefCell<T>; // Should be linted at definition site
}

struct Dummy;

impl<T: Copy, U> Trait<T, U> for Dummy {
    type NG = RefCell<T>; //~ refcell_cell
    type OK = RefCell<U>;

    // Should be linted at definition site
    fn ng(ng: RefCell<T>) {}
    fn ok(ok: RefCell<U>) {}
    fn ng_ref(ng: &RefCell<T>) {}
    fn ok_ref(ok: &RefCell<U>) {}
    fn ng_ret() -> RefCell<T> {
        todo!()
    }
    fn ok_ret() -> RefCell<U> {
        todo!()
    }
    fn ok_ret_ref<'a>() -> &'a RefCell<T> {
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
    &'a RefCell<T>, // Should be linted at definition site
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
    // Simple constructors
    {
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
    }

    // Macro expansion
    {
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
}

// Suppress false positives for the simple ctor pattern
mod usage_analysis {
    use std::cell::RefCell;
    use std::hint::black_box;

    fn callee<T>(x: RefCell<T>) -> T {
        todo!()
    }

    fn callee_ref<T>(x: &RefCell<T>) -> T {
        todo!()
    }

    fn check_calls() {
        {
            let x = RefCell::new(1); //~ refcell_cell
            let _ = black_box(x);
        }
        {
            let x = RefCell::new(1); //~ refcell_cell
            let _ = black_box(&x);
        }
        // Should be linted at definition of `callee` and `callee_ref`
        {
            let x = RefCell::new(1);
            let _ = callee(x);
        }
        {
            let x = RefCell::new(1);
            let _ = callee_ref(&x);
        }
    }

    struct MethodCall {}

    impl MethodCall {
        fn black_box<T>(&self, x: T) -> T {
            todo!()
        }

        fn callee<T>(&self, x: RefCell<T>) -> T {
            todo!()
        }

        fn callee_ref<T>(&self, x: &RefCell<T>) -> T {
            todo!()
        }
    }

    fn check_method_calls() {
        let m = MethodCall {};

        {
            let x = RefCell::new(1); //~ refcell_cell
            let _ = m.black_box(x);
        }
        {
            let x = RefCell::new(1); //~ refcell_cell
            let _ = m.black_box(&x);
        }
        // Should be linted at definition of `callee` and `callee_ref`
        {
            let x = RefCell::new(1);
            let _ = m.callee(x);
        }
        {
            let x = RefCell::new(1);
            let _ = m.callee_ref(&x);
        }
    }
}

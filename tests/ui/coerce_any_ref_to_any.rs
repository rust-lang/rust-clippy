#![warn(clippy::coerce_any_ref_to_any)]

use std::any::Any;

fn main() {
    let x: Box<dyn Any> = Box::new(());
    let ref_x = &x;

    f(&x);
    //~^ coerce_any_ref_to_any

    f(ref_x);
    //~^ coerce_any_ref_to_any

    let _: &dyn Any = &x;
    //~^ coerce_any_ref_to_any

    f(&42);
    f(&Box::new(()));
    f(&**ref_x);
    f(&*x);
    let _: &dyn Any = &*x;
}

fn f(_: &dyn Any) {}

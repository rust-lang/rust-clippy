#![warn(clippy::unit_as_impl_trait)]
#![allow(clippy::unused_unit)]

fn implicit_unit() -> impl Copy {
    //~^ ERROR: function returns `()` which implements the required trait
}

fn explicit_unit() -> impl Copy {
    ()
}

fn not_unit(x: u32) -> impl Copy {
    x
}

fn never(x: u32) -> impl Copy {
    panic!();
}

fn with_generic_param<T: Eq>(x: T) -> impl Eq {
    //~^ ERROR: function returns `()` which implements the required trait
    x;
}

fn non_empty_implicit_unit() -> impl Copy {
    //~^ ERROR: function returns `()` which implements the required trait
    println!("foobar");
}

fn last_expression_returning_unit() -> impl Eq {
    //~^ ERROR: function returns `()` which implements the required trait
    [1, 10, 2, 0].sort_unstable()
}

#[derive(Clone)]
struct S;

impl S {
    fn clone(&self) -> impl Clone {
        //~^ ERROR: function returns `()` which implements the required trait
        S;
    }
}

fn main() {}

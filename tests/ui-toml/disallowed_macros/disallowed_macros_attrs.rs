//@aux-build:macros.rs

// Disallowed macros whose expansion contains attributes used to be linted at the crate level, so
// `#[allow]`/`#[expect]` on the function or module using them had no effect.

extern crate macros;

// Issue #14017
// `panic!("{}", x)` used to expand to a helper function with attributes (`#[cold]`, ...). It no
// longer does, but these cases are kept as a regression test. `println!` and `vec!` are included
// for comparison.
#[allow(clippy::disallowed_macros)]
fn allow_on_function() {
    panic!();
    panic!("message");
    panic!("{}", 1);
    panic!("{} {}", 1, 2);
    println!("test");
    println!("{}", 1);
    vec![1, 2, 3];
}

#[expect(clippy::disallowed_macros)]
fn expect_on_function() {
    panic!();
    panic!("message");
    panic!("{}", 1);
    panic!("{} {}", 1, 2);
    println!("test");
    println!("{}", 1);
    vec![1, 2, 3];
}

fn no_attributes_function() {
    panic!();
    //~^ disallowed_macros
    panic!("message");
    //~^ disallowed_macros
    panic!("{}", 1);
    //~^ disallowed_macros
    panic!("{} {}", 1, 2);
    //~^ disallowed_macros
    println!("test");
    //~^ disallowed_macros
    println!("{}", 1);
    //~^ disallowed_macros
    vec![1, 2, 3];
    //~^ disallowed_macros
}

#[allow(clippy::disallowed_macros)]
mod allow_on_module {
    pub fn f() {
        panic!();
        panic!("message");
        panic!("{}", 1);
        panic!("{} {}", 1, 2);
        println!("test");
        println!("{}", 1);
        vec![1, 2, 3];
    }
}

#[expect(clippy::disallowed_macros)]
mod expect_on_module {
    pub fn f() {
        panic!();
        panic!("message");
        panic!("{}", 1);
        panic!("{} {}", 1, 2);
        println!("test");
        println!("{}", 1);
        vec![1, 2, 3];
    }
}

mod no_attributes_module {
    pub fn f() {
        panic!();
        //~^ disallowed_macros
        panic!("message");
        //~^ disallowed_macros
        panic!("{}", 1);
        //~^ disallowed_macros
        panic!("{} {}", 1, 2);
        //~^ disallowed_macros
        println!("test");
        //~^ disallowed_macros
        println!("{}", 1);
        //~^ disallowed_macros
        vec![1, 2, 3];
        //~^ disallowed_macros
    }
}

// Issue #15312
// A disallowed macro expanding to an item with an attribute. Unlike `macros::attr!`, which only adds
// an attribute to an item written by the caller, the whole item comes from the macro, so
// `#[allow]`/`#[expect]` also work when it is used directly in a module.
mod attr_item {
    #[allow(clippy::disallowed_macros)]
    fn allow_on_function() {
        macros::attr_item!();
    }

    #[expect(clippy::disallowed_macros)]
    fn expect_on_function() {
        macros::attr_item!();
    }

    fn no_attributes_function() {
        macros::attr_item!();
        //~^ disallowed_macros
    }

    #[allow(clippy::disallowed_macros)]
    mod allow_on_module {
        macros::attr_item!();
    }

    #[expect(clippy::disallowed_macros)]
    mod expect_on_module {
        macros::attr_item!();
    }

    mod no_attributes_module {
        macros::attr_item!();
        //~^ disallowed_macros
    }
}

// Issue #15312
// A wrapper macro that puts `#[allow(clippy::disallowed_macros)]` on its use of a disallowed macro
// whose expansion contains an attribute. Using the wrapper must not lint.
mod wrapper {
    fn wrapped() -> i32 {
        macros::wrap_attr_expr!(1)
    }

    fn unwrapped() -> i32 {
        macros::attr_expr!(1)
        //~^ disallowed_macros
    }
}

// The expansion of `#[derive(Serialize)]` contains attributes (e.g. `#[doc(hidden)]`) too.
mod derive {
    use serde::Serialize;

    #[allow(clippy::disallowed_macros)]
    #[derive(Serialize)]
    struct Allowed;

    #[expect(clippy::disallowed_macros)]
    #[derive(Serialize)]
    struct Expected;

    #[derive(Serialize)]
    //~^ disallowed_macros
    struct NoAttributes;
}

fn main() {}

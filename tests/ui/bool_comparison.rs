#![allow(clippy::needless_if)]
#![warn(clippy::bool_comparison)]
#![allow(clippy::incorrect_partial_ord_impl_on_ord_type)]

fn main() {
    let x = true;
    if x == true {
    //~^ ERROR: equality checks against true are unnecessary
    //~| NOTE: `-D clippy::bool-comparison` implied by `-D warnings`
        "yes"
    } else {
        "no"
    };
    if x == false {
    //~^ ERROR: equality checks against false can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    if true == x {
    //~^ ERROR: equality checks against true are unnecessary
        "yes"
    } else {
        "no"
    };
    if false == x {
    //~^ ERROR: equality checks against false can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    if x != true {
    //~^ ERROR: inequality checks against true can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    if x != false {
    //~^ ERROR: inequality checks against false are unnecessary
        "yes"
    } else {
        "no"
    };
    if true != x {
    //~^ ERROR: inequality checks against true can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    if false != x {
    //~^ ERROR: inequality checks against false are unnecessary
        "yes"
    } else {
        "no"
    };
    if x < true {
    //~^ ERROR: less than comparison against true can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    if false < x {
    //~^ ERROR: greater than checks against false are unnecessary
        "yes"
    } else {
        "no"
    };
    if x > false {
    //~^ ERROR: greater than checks against false are unnecessary
        "yes"
    } else {
        "no"
    };
    if true > x {
    //~^ ERROR: less than comparison against true can be replaced by a negation
        "yes"
    } else {
        "no"
    };
    let y = true;
    if x < y {
    //~^ ERROR: order comparisons between booleans can be simplified
        "yes"
    } else {
        "no"
    };
    if x > y {
    //~^ ERROR: order comparisons between booleans can be simplified
        "yes"
    } else {
        "no"
    };
}

#[allow(dead_code)]
fn issue3703() {
    struct Foo;
    impl PartialEq<bool> for Foo {
        fn eq(&self, _: &bool) -> bool {
            true
        }
    }
    impl PartialEq<Foo> for bool {
        fn eq(&self, _: &Foo) -> bool {
            true
        }
    }
    impl PartialOrd<bool> for Foo {
        fn partial_cmp(&self, _: &bool) -> Option<std::cmp::Ordering> {
            None
        }
    }
    impl PartialOrd<Foo> for bool {
        fn partial_cmp(&self, _: &Foo) -> Option<std::cmp::Ordering> {
            None
        }
    }

    if Foo == true {}
    if true == Foo {}
    if Foo != true {}
    if true != Foo {}
    if Foo == false {}
    if false == Foo {}
    if Foo != false {}
    if false != Foo {}
    if Foo < false {}
    if false < Foo {}
}

#[allow(dead_code)]
fn issue4983() {
    let a = true;
    let b = false;

    if a == !b {};
    //~^ ERROR: this comparison might be written more concisely
    if !a == b {};
    //~^ ERROR: this comparison might be written more concisely
    if a == b {};
    if !a == !b {};

    if b == !a {};
    //~^ ERROR: this comparison might be written more concisely
    if !b == a {};
    //~^ ERROR: this comparison might be written more concisely
    if b == a {};
    if !b == !a {};
}

macro_rules! m {
    ($func:ident) => {
        $func()
    };
}

fn func() -> bool {
    true
}

#[allow(dead_code)]
fn issue3973() {
    // ok, don't lint on `cfg` invocation
    if false == cfg!(feature = "debugging") {}
    if cfg!(feature = "debugging") == false {}
    if true == cfg!(feature = "debugging") {}
    if cfg!(feature = "debugging") == true {}

    // lint, could be simplified
    if false == m!(func) {}
    //~^ ERROR: equality checks against false can be replaced by a negation
    if m!(func) == false {}
    //~^ ERROR: equality checks against false can be replaced by a negation
    if true == m!(func) {}
    //~^ ERROR: equality checks against true are unnecessary
    if m!(func) == true {}
    //~^ ERROR: equality checks against true are unnecessary

    // no lint with a variable
    let is_debug = false;
    if is_debug == cfg!(feature = "debugging") {}
    if cfg!(feature = "debugging") == is_debug {}
    if is_debug == m!(func) {}
    if m!(func) == is_debug {}
    let is_debug = true;
    if is_debug == cfg!(feature = "debugging") {}
    if cfg!(feature = "debugging") == is_debug {}
    if is_debug == m!(func) {}
    if m!(func) == is_debug {}
}

#![warn(clippy::pedantic)]
#![allow(
    unused,
    clippy::let_and_return,
    clippy::redundant_closure_call,
    clippy::no_effect,
    clippy::unnecessary_operation,
    clippy::needless_if,
    clippy::single_match,
    clippy::unused_self
)]

fn main() {
    // This should not trigger with default threshold of 6
    let a = {
        let b = {
            let c = {
                let d = {
                    let e = {
                        let f = { 42 };
                        //~^ ERROR: this block is too nested
                        f
                    };
                    e
                };
                d
            };
            c
        };
        b
    };

    // This should trigger with default threshold of 6
    let x = {
        let y = {
            let z = {
                let w = {
                    let v = {
                        let u = {
                            //~^ ERROR: this block is too nested
                            let t = { 42 };
                            t
                        };
                        u
                    };
                    v
                };
                w
            };
            z
        };
        y
    };
}

struct A;

impl A {
    fn test() {
        // This should not trigger
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        if true {
                            //~^ ERROR: this block is too nested
                            let x = { 1 };
                        }
                    }
                }
            }
        }
    }
}

trait TestTrait {
    fn test() {
        // This should trigger (7 levels)
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        if true {
                            //~^ ERROR: this block is too nested
                            let x = { 1 };
                        }
                    }
                }
            }
        }
    }
}

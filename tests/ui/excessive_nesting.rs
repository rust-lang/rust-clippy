#![warn(clippy::pedantic)]
#![allow(clippy::let_and_return)]

fn main() {
    // This should trigger at level 7
    let a = {
        let b = {
            let c = {
                let d = {
                    let e = {
                        let f = { 42 };
                        //~^ excessive_nesting
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

    // This should trigger at level 7
    let x = {
        let y = {
            let z = {
                let w = {
                    let v = {
                        let u = {
                            //~^ excessive_nesting
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
        // This should trigger at level 7
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        if true {
                            //~^ excessive_nesting
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
        // This should trigger at level 7
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        if true {
                            //~^ excessive_nesting
                            let x = { 1 };
                        }
                    }
                }
            }
        }
    }
}

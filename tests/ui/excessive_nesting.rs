#![warn(clippy::pedantic)]
#![allow(clippy::let_and_return)]

fn main() {
    // This should not trigger
    let a = {
        let b = {
            let c = {
                let d = {
                    let e = {
                        let f = { 42 };
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

    // This should trigger at level 9
    let x = {
        let y = {
            let z = {
                let w = {
                    let v = {
                        let u = {
                            let t = {
                                let s = { 42 };
                                //~^ excessive_nesting
                                s
                            };
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
        // This should trigger at level 9
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        {
                            if true {
                                let x = { 1 };
                                //~^ excessive_nesting
                            }
                        }
                    }
                }
            }
        }
    }
}

trait TestTrait {
    fn test() {
        // This should trigger at level 9
        struct B;
        impl B {
            fn test() {
                struct C;
                impl C {
                    fn test() {
                        {
                            if true {
                                let x = { 1 };
                                //~^ excessive_nesting
                            }
                        }
                    }
                }
            }
        }
    }
}

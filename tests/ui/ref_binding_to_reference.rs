#![warn(clippy::ref_binding_to_reference)]
#![expect(clippy::collapsible_match, clippy::explicit_auto_deref, clippy::never_loop)]
#![allow(clippy::needless_borrowed_reference)]

fn f1(_: &str) {}
macro_rules! m2 {
    ($e:expr) => {
        f1(*$e)
    };
}
macro_rules! m3 {
    ($i:ident) => {
        Some(ref $i)
    };
}

fn main() {
    let x = String::new();

    // Ok, the pattern is from a macro
    let _: &&String = match Some(&x) {
        m3!(x) => x,
        None => return,
    };

    let _: &&String = match Some(&x) {
        Some(ref x) => x,
        None => return,
    };

    let _: &&String = match Some(&x) {
        Some(ref x) => {
            f1(x);
            f1(*x);
            x
        },
        None => return,
    };

    // Err, reference to a &String
    match Some(&x) {
        Some(ref x) => m2!(x),
        //~^ ref_binding_to_reference
        None => return,
    }

    // Err, reference to a &String
    let _ = |&ref x: &&String| {
        //~^ ref_binding_to_reference

        let _: &&String = x;
    };
}

// Err, reference to a &String
fn f2<'a>(&ref x: &&'a String) -> &'a String {
    //~^ ref_binding_to_reference

    let _: &&String = x;
    *x
}

trait T1 {
    // Err, reference to a &String
    fn f(&ref x: &&String) {
        //~^ ref_binding_to_reference

        let _: &&String = x;
    }
}

struct S;
impl T1 for S {
    // Err, reference to a &String
    fn f(&ref x: &&String) {
        //~^ ref_binding_to_reference

        let _: &&String = x;
    }
}

mod issue17370 {
    fn f1(_: &str) {}

    fn match_ref_some() {
        let x = String::new();
        let _: &&String = match Some(&x) {
            Some(ref x) => x,
            None => return,
        };
    }

    fn match_ref(x: String) {
        let _: &&String = match &x {
            ref x => x,
            _ => return,
        };
    }

    fn if_let(x: String) {
        let opt = Some(&x);

        let _: &&String = if let Some(ref x) = opt {
            x
        } else {
            return;
        };
    }

    fn if_let_block_tail(x: String) {
        let opt = Some(&x);

        let _: &&String = if let Some(ref x) = opt {
            f1(x);
            f1(*x);
            x
        } else {
            return;
        };
    }

    fn normal_if_tail(x: String) {
        let _: &&String = match Some(&x) {
            Some(ref x) => {
                if true {
                    x
                } else {
                    return;
                }
            },
            None => return,
        };
    }

    fn normal_if_block_tail(x: String) {
        let _: &&String = match Some(&x) {
            Some(ref x) => {
                if true {
                    f1(x);
                    f1(*x);
                    x
                } else {
                    return;
                }
            },
            None => return,
        };
    }

    fn normal_else_tail(x: String) {
        let _: &&String = match Some(&x) {
            Some(ref x) => {
                if true {
                    return;
                } else {
                    x
                }
            },
            None => return,
        };
    }

    fn normal_else_block_tail(x: String) {
        let _: &&String = match Some(&x) {
            Some(ref x) => {
                if true {
                    return;
                } else {
                    f1(x);
                    f1(*x);
                    x
                }
            },
            None => return,
        };
    }

    fn break_if_let(x: String) {
        let opt = Some(&x);

        let _: &&String = loop {
            if let Some(ref x) = opt {
                break x;
            } else {
                return;
            }
        };
    }

    fn break_match(x: String) {
        let opt = Some(&x);

        let _: &&String = loop {
            match opt {
                Some(ref x) => break x,
                None => return,
            }
        };
    }

    fn if_let_chain(x: String) {
        let opt = Some(&x);

        let _: &&String = if true && let Some(ref x) = opt {
            x
        } else {
            return;
        };
    }

    fn if_let_chain_block_tail(x: String) {
        let opt = Some(&x);

        let _: &&String = if true && let Some(ref x) = opt {
            f1(x);
            f1(*x);
            x
        } else {
            return;
        };
    }
}

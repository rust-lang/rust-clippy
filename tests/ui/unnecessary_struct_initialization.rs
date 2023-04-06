// run-rustfix

#![allow(unused)]
#![warn(clippy::unnecessary_struct_initialization)]

struct S {
    f: String,
}

#[derive(Clone, Copy)]
struct T {
    f: u32,
}

struct U {
    f: u32,
}

impl Clone for U {
    fn clone(&self) -> Self {
        // Do not lint: `Self` does not implement `Copy`
        Self { ..*self }
    }
}

#[derive(Copy)]
struct V {
    f: u32,
}

impl Clone for V {
    fn clone(&self) -> Self {
        // Lint: `Self` implements `Copy`
        Self { ..*self }
    }
}

fn main() {
    // Should lint: `a` would be consumed anyway
    let a = S { f: String::from("foo") };
    let mut b = S { ..a };

    // Should lint: `b` would be consumed, and is mutable
    let c = &mut S { ..b };

    // Should not lint as `d` is not mutable
    let d = S { f: String::from("foo") };
    let e = &mut S { ..d };

    // Should lint as `f` would be consumed anyway
    let f = S { f: String::from("foo") };
    let g = &S { ..f };

    // Should lint: the result of an expression is mutable
    let h = &mut S {
        ..*Box::new(S { f: String::from("foo") })
    };

    // Should not lint: `m` would be both alive and borrowed
    let m = T { f: 17 };
    let n = &T { ..m };

    // Should not lint: `m` should not be modified
    let o = &mut T { ..m };
    o.f = 32;
    assert_eq!(m.f, 17);

    // Should not lint: `m` should not be modified
    let o = &mut T { ..m } as *mut T;
    unsafe { &mut *o }.f = 32;
    assert_eq!(m.f, 17);

    // Should lint: the result of an expression is mutable and temporary
    let p = &mut T {
        ..*Box::new(T { f: 5 })
    };
}

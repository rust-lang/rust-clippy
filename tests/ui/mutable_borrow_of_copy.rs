#![warn(clippy::mutable_borrow_of_copy)]
#![allow(clippy::deref_addrof)]

fn main() {
    let mut a = [0u8; 2];
    let _ = &mut { a }; //~ mutable_borrow_of_copy

    let _ = &mut 'label: {
        // Block is targeted by break
        if a[0] == 1 {
            break 'label 42u8;
        }
        a[0]
    };

    let mut a = vec![0u8; 2];
    let _ = &mut { a }; // `a` is not `Copy`

    let a = [0u8; 2];
    let _ = &mut { a }; // `a` is not mutable

    let _ = &mut { 42 }; // Do not lint on non-place expression

    let _ = &mut {}; // Do not lint on empty block

    macro_rules! mac {
        ($a:expr) => {{ a }};
    }
    let _ = &mut mac!(a); // Do not lint on borrowed macro result

    macro_rules! mac2 {
        // Do not lint, as it depends on `Copy`-ness of `a`
        ($x:expr) => {
            &mut unsafe { $x }
        };
    }
    let mut a = 0u8;
    let _ = &mut mac2!(a);

    let _ = &mut {
        // Do not lint, the variable is defined inside the block
        let mut a: [i32; 5] = (1, 2, 3, 4, 5).into();
        a
    };

    let _ = &mut {
        // Do not lint, the variable is defined inside the block
        {
            let mut a: [i32; 5] = (1, 2, 3, 4, 5).into();
            a
        }
    };

    struct S {
        a: u32,
    }

    let mut s = S { a: 0 };
    let _ = &mut {
        //~^ mutable_borrow_of_copy
        s.a = 32;
        s.a
    };

    let _ = &mut {
        // Do not lint, the variable is defined inside the block
        let mut s = S { a: 0 };
        s.a
    };

    let mut c = (10, 20);
    let _ = &mut {
        //~^ ERROR: mutable borrow
        c.0
    };

    let _ = &mut {
        // Do not lint, the variable is defined inside the block
        let mut c = (10, 20);
        c.0
    };

    let mut t = [10, 20];
    let _ = &mut {
        //~^ ERROR: mutable borrow
        t[0]
    };

    let _ = &mut {
        // Do not lint, the variable is defined inside the block
        let mut t = [10, 20];
        t[0]
    };

    unsafe fn unsafe_func(_: &mut i32) {}
    let mut a = 10;
    // Unsafe block needed to call `unsafe_func`
    let double_a_ref = &mut unsafe {
        //~^ ERROR: mutable borrow
        unsafe_func(&mut a);
        a
    };
}

#[test]
fn in_test() {
    let mut a = [10; 2];
    let _ = &mut { a }; //~ ERROR: mutable borrow
}

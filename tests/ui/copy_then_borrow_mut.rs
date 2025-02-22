//@no-rustfix
#![warn(clippy::copy_then_borrow_mut)]

fn main() {
    let a = &[0u8; 2];
    let _ = &mut { *a }; //~ ERROR: mutable borrow

    let a = [0u8; 2];
    let _ = &mut { a }; //~ ERROR: mutable borrow
    let _ = &mut { *{ &a } }; //~ ERROR: mutable borrow

    let _ = &mut 'label: {
        //~^ ERROR: mutable borrow
        if a[0] == 1 {
            break 'label 42u8;
        }
        10u8
    };

    let a = vec![0u8; 2];
    let _ = &mut { a }; // `a` is not `Copy`

    let a: *const u8 = &0u8;
    let _ = &mut { unsafe { *a } }; //~ ERROR: mutable borrow

    let _ = &mut {}; // Do not lint on empty block

    macro_rules! mac {
        () => {{ 0u8 }};
    }
    let _ = &mut mac!(); // Do not lint on borrowed macro result

    macro_rules! mac2 {
        // Do not lint, as it depends on `Copy`-ness of `a`
        ($x:expr) => {
            &mut unsafe { *$x }
        };
    }
    let a = 0u8;
    let _ = &mut mac2!(&raw const a);
}

// From https://users.rust-lang.org/t/runtime-speed-when-converting-as-bytes-or-as-ref-to-be-mut/123712/10
fn f<N: Default + AsRef<T>, T: Copy>() {
    let _: &mut T = {
        let n = N::default();
        &mut { *n.as_ref() } //~ ERROR: mutable borrow
    };
}

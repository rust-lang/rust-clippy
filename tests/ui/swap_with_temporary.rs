#![warn(clippy::swap_with_temporary)]

use std::mem::swap;

fn func() -> String {
    String::from("func")
}

fn func_returning_refmut(s: &mut String) -> &mut String {
    s
}

fn main() {
    let mut x = String::from("x");
    let mut y = String::from("y");
    let mut zz = String::from("zz");
    let z = &mut zz;

    // No lint
    swap(&mut x, &mut y);

    swap(&mut func(), &mut func());
    //~^ ERROR: swapping temporary values has no effect

    swap(&mut func(), &mut y);
    //~^ ERROR: swapping with a temporary value is inefficient

    swap(&mut x, &mut func());
    //~^ ERROR: swapping with a temporary value is inefficient

    swap(z, &mut func());
    //~^ ERROR: swapping with a temporary value is inefficient

    // No lint
    swap(z, func_returning_refmut(&mut x));

    swap(&mut y, z);

    swap(&mut func(), z);
    //~^ ERROR: swapping with a temporary value is inefficient

    // Check rewrites as part of a larger expression
    if matches!(swap(&mut func(), &mut func()), ()) {
        //~^ ERROR: swapping temporary values has no effect
        println!("Yeah");
    }
    if matches!(swap(z, &mut func()), ()) {
        //~^ ERROR: swapping with a temporary value is inefficient
        println!("Yeah");
    }

    macro_rules! mac {
        (refmut $x:expr) => {
            &mut $x
        };
        (funcall $f:ident) => {
            $f()
        };
        (wholeexpr) => {
            swap(&mut 42, &mut 0)
        };
        (ident $v:ident) => {
            $v
        };
    }
    swap(mac!(refmut func()), z);
    //~^ ERROR: swapping with a temporary value is inefficient
    swap(&mut mac!(funcall func), z);
    //~^ ERROR: swapping with a temporary value is inefficient
    swap(&mut mac!(funcall func), mac!(ident z));
    //~^ ERROR: swapping with a temporary value is inefficient
    swap(mac!(refmut y), &mut func());
    //~^ ERROR: swapping with a temporary value is inefficient

    // No lint if it comes from a macro as it may depend on the arguments
    mac!(wholeexpr);
}

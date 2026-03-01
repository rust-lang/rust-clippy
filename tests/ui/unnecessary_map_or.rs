//@aux-build:proc_macros.rs
#![warn(clippy::unnecessary_map_or, clippy::manual_is_variant_and)]
#![allow(clippy::no_effect)]
#![allow(clippy::eq_op)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::nonminimal_bool)]
#[clippy::msrv = "1.82.0"]
#[macro_use]
extern crate proc_macros;

fn main() {
    // should trigger
    let _ = Some(5).map_or(false, |n| n == 5);
    //~^ unnecessary_map_or
    let _ = Some(5).map_or(true, |n| n != 5);
    //~^ unnecessary_map_or
    let _ = Some(5).map_or(false, |n| {
        //~^ unnecessary_map_or
        let _ = 1;
        n == 5
    });
    let _ = Ok::<i32, i32>(5).map_or(false, |n| n == 5);
    //~^ unnecessary_map_or
    let _ = Some(5).map_or(false, |n| n == 5).then(|| 1);
    //~^ unnecessary_map_or
    let _ = !Some(5).map_or(false, |n| n == 5);
    //~^ unnecessary_map_or
    let _ = Some(5).map_or(false, |n| n == 5) || false;
    //~^ unnecessary_map_or
    let _ = Some(5).map_or(false, |n| n == 5) as usize;
    //~^ unnecessary_map_or

    macro_rules! x {
        () => {
            Some(1)
        };
    }
    // methods lints dont fire on macros
    let _ = x!().map_or(false, |n| n == 1);
    let _ = x!().map_or(false, |n| n == vec![1][0]);

    external! {
        let _ = Some(5).map_or(false, |n| n == 5);
    }

    with_span! {
        let _ = Some(5).map_or(false, |n| n == 5);
    }

    // do not lint in absense of PartialEq (covered by `manual_is_variant_and`)
    struct S;
    let r: Result<i32, S> = Ok(3);
    let _ = r.map_or(false, |x| x == 7);
    //~^ manual_is_variant_and

    #[derive(PartialEq)]
    struct S2;
    let r: Result<i32, S2> = Ok(4);
    let _ = r.map_or(false, |x| x == 8);
    //~^ unnecessary_map_or

    // do not lint constructs that are not comparisons (covered by `manual_is_variant_and`)
    let func = |_x| true;
    let r: Result<i32, S> = Ok(3);
    let _ = r.map_or(false, func);
    //~^ manual_is_variant_and
    let _ = Some(5).map_or(false, func);
    //~^ manual_is_variant_and
    let _ = Some(5).map_or(true, func);
    //~^ manual_is_variant_and
}

fn issue14714() {
    assert!(Some("test").map_or(false, |x| x == "test"));
    //~^ unnecessary_map_or

    // even though we're in a macro context, we still need to parenthesise because of the `then`
    assert!(Some("test").map_or(false, |x| x == "test").then(|| 1).is_some());
    //~^ unnecessary_map_or

    // method lints don't fire on macros
    macro_rules! m {
        ($x:expr) => {
            // should become !($x == Some(1))
            let _ = !$x.map_or(false, |v| v == 1);
            // should become $x == Some(1)
            let _ = $x.map_or(false, |v| v == 1);
        };
    }
    m!(Some(5));
}

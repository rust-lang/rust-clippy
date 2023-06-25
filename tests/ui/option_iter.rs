//@run-rustfix
//@aux-build:proc_macros.rs
#![allow(clippy::needless_return, clippy::no_effect, unused)]
#![warn(clippy::option_iter)]

#[macro_use]
extern crate proc_macros;

fn returned_1() -> impl IntoIterator<Item = i32> {
    Some(0)
}

fn returned_2() -> impl IntoIterator<Item = i32> {
    return std::iter::once(0);
}

fn returned_4() -> impl IntoIterator<Item = i32> {
    std::iter::empty::<i32>()
}

fn returned_5() -> impl IntoIterator<Item = i32> {
    None
}

fn main() {
    let x = Some(1).iter();
    let x = Some(1).iter_mut();
    let x = Some(1).into_iter();
    let x = None::<()>.iter();
    let x = None::<()>.iter_mut();
    let x = None::<()>.into_iter();
    let x = vec![0].into_iter().chain(Some(0));
    let x = vec![0].into_iter().zip(Some(0));
    let x = vec![0].into_iter().eq(Some(0));
    let x = vec![0].into_iter().ne(Some(0));
    let x = vec![0].into_iter().lt(Some(0));
    // Don't lint
    external! {
        let x = Some(1).into_iter();
        let x = None::<()>.into_iter();
    }
    with_span! {
        span
        let x = Some(1).into_iter();
        let x = None::<()>.into_iter();
    }
}

fn not_returned_1() -> impl IntoIterator<Item = i32> {
    Some(0);
    std::iter::once(0)
}

fn not_returned_2() {
    Some(0);
}

fn not_returned_3() -> Option<i32> {
    Some(0)
}

// FPs:
/*
fn returned_3() -> impl IntoIterator<Item = i32> {
    return Some(0);
    None::<i32>
}

fn returned_6() -> impl IntoIterator<Item = i32> {
    {
        return Some(0);
    }
    None
}

fn for_loop_ref() {
    for &x in Some(Some(true)).iter() {
        let _ = match x {
            Some(x) => Some(if x { continue } else { x }),
            None => None,
        };
    }
}
*/
// Okay, that last one isn't a FP but the suggestion is incorrect (&x -> x). Let's fix these :)

#[clippy::msrv = "1.1.0"]
fn msrv_too_low() {
    let x = Some(1).into_iter();
    let x = None::<()>.into_iter();
}

#[clippy::msrv = "1.2.0"]
fn msrv_juust_right() {
    let x = Some(1).into_iter();
    let x = None::<()>.into_iter();
}

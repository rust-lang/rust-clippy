#![warn(clippy::all)]
#![warn(clippy::redundant_pattern_matching)]
#![allow(
    unused_must_use,
    clippy::needless_bool,
    clippy::needless_if,
    clippy::match_like_matches_macro,
    clippy::equatable_if_let,
    clippy::if_same_then_else
)]
#![feature(let_chains, if_let_guard)]

fn issue_11174<T>(boolean: bool, maybe_some: Option<T>) -> bool {
    matches!(maybe_some, None if !boolean)
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
    //~| NOTE: `-D clippy::redundant-pattern-matching` implied by `-D warnings`
}

fn issue_11174_edge_cases<T>(boolean: bool, boolean2: bool, maybe_some: Option<T>) {
    let _ = matches!(maybe_some, None if boolean || boolean2); // guard needs parentheses
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
    let _ = match maybe_some { // can't use `matches!` here
                               // because `expr` metavars in macros don't allow let exprs
        None if let Some(x) = Some(0) && x > 5 => true,
        _ => false
    };
}

fn main() {
    if let None = None::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    if let Some(_) = Some(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    if let Some(_) = Some(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        foo();
    } else {
        bar();
    }

    while let Some(_) = Some(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    while let None = Some(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    while let None = None::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    let mut v = vec![1, 2, 3];
    while let Some(_) = v.pop() {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        foo();
    }

    if None::<i32>.is_none() {}

    if Some(42).is_some() {}

    match Some(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        Some(_) => true,
        None => false,
    };

    match None::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        Some(_) => false,
        None => true,
    };

    let _ = match None::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        Some(_) => false,
        None => true,
    };

    let opt = Some(false);
    let _ = if let Some(_) = opt { true } else { false };
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    issue6067();
    issue10726();
    issue10803();

    let _ = if let Some(_) = gen_opt() {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        1
    } else if let None = gen_opt() {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        2
    } else {
        3
    };

    if let Some(..) = gen_opt() {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
}

fn gen_opt() -> Option<()> {
    None
}

fn foo() {}

fn bar() {}

// Methods that are unstable const should not be suggested within a const context, see issue #5697.
// However, in Rust 1.48.0 the methods `is_some` and `is_none` of `Option` were stabilized as const,
// so the following should be linted.
const fn issue6067() {
    if let Some(_) = Some(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    if let None = None::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    while let Some(_) = Some(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    while let None = None::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    match Some(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        Some(_) => true,
        None => false,
    };

    match None::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        Some(_) => false,
        None => true,
    };
}

#[allow(clippy::deref_addrof, dead_code, clippy::needless_borrow)]
fn issue7921() {
    if let None = *(&None::<()>) {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
    if let None = *&None::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
}

fn issue10726() {
    let x = Some(42);

    match x {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        Some(_) => true,
        _ => false,
    };

    match x {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        None => true,
        _ => false,
    };

    match x {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
        Some(_) => false,
        _ => true,
    };

    match x {
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
        None => false,
        _ => true,
    };

    // Don't lint
    match x {
        Some(21) => true,
        _ => false,
    };
}

fn issue10803() {
    let x = Some(42);

    let _ = matches!(x, Some(_));
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    let _ = matches!(x, None);
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    // Don't lint
    let _ = matches!(x, Some(16));
}

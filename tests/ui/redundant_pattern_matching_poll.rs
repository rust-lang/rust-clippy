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

use std::task::Poll::{self, Pending, Ready};

fn main() {
    if let Pending = Pending::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
    //~| NOTE: `-D clippy::redundant-pattern-matching` implied by `-D warnings`

    if let Ready(_) = Ready(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`

    if let Ready(_) = Ready(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
        foo();
    } else {
        bar();
    }

    while let Ready(_) = Ready(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`

    while let Pending = Ready(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`

    while let Pending = Pending::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`

    if Pending::<i32>.is_pending() {}

    if Ready(42).is_ready() {}

    match Ready(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
        Ready(_) => true,
        Pending => false,
    };

    match Pending::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
        Ready(_) => false,
        Pending => true,
    };

    let _ = match Pending::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
        Ready(_) => false,
        Pending => true,
    };

    let poll = Ready(false);
    let _ = if let Ready(_) = poll { true } else { false };
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`

    poll_const();

    let _ = if let Ready(_) = gen_poll() {
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
        1
    } else if let Pending = gen_poll() {
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
        2
    } else {
        3
    };
}

fn gen_poll() -> Poll<()> {
    Pending
}

fn foo() {}

fn bar() {}

const fn poll_const() {
    if let Ready(_) = Ready(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`

    if let Pending = Pending::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`

    while let Ready(_) = Ready(42) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`

    while let Pending = Pending::<()> {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`

    match Ready(42) {
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
        Ready(_) => true,
        Pending => false,
    };

    match Pending::<()> {
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
        Ready(_) => false,
        Pending => true,
    };
}

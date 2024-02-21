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
    if let Pending = Pending::<()> {} //~ redundant_pattern_matching

    if let Ready(_) = Ready(42) {} //~ redundant_pattern_matching

    //~v redundant_pattern_matching
    if let Ready(_) = Ready(42) {
        foo();
    } else {
        bar();
    }

    // Issue 6459
    if matches!(Ready(42), Ready(_)) {}
    //~^ redundant_pattern_matching

    // Issue 6459
    if matches!(Pending::<()>, Pending) {}
    //~^ redundant_pattern_matching

    while let Ready(_) = Ready(42) {} //~ redundant_pattern_matching

    while let Pending = Ready(42) {} //~ redundant_pattern_matching

    while let Pending = Pending::<()> {}
    //~^ redundant_pattern_matching

    if Pending::<i32>.is_pending() {}

    if Ready(42).is_ready() {}

    //~v redundant_pattern_matching
    match Ready(42) {
        Ready(_) => true,
        Pending => false,
    };

    //~v redundant_pattern_matching
    match Pending::<()> {
        Ready(_) => false,
        Pending => true,
    };

    //~v redundant_pattern_matching
    let _ = match Pending::<()> {
        Ready(_) => false,
        Pending => true,
    };

    let poll = Ready(false);
    let _ = if let Ready(_) = poll { true } else { false };
    //~^ redundant_pattern_matching

    poll_const();

    //~v redundant_pattern_matching
    let _ = if let Ready(_) = gen_poll() {
        1
    //~v redundant_pattern_matching
    } else if let Pending = gen_poll() {
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
    if let Ready(_) = Ready(42) {} //~ redundant_pattern_matching

    if let Pending = Pending::<()> {} //~ redundant_pattern_matching

    while let Ready(_) = Ready(42) {} //~ redundant_pattern_matching

    while let Pending = Pending::<()> {}
    //~^ redundant_pattern_matching

    //~v redundant_pattern_matching
    match Ready(42) {
        Ready(_) => true,
        Pending => false,
    };

    //~v redundant_pattern_matching
    match Pending::<()> {
        Ready(_) => false,
        Pending => true,
    };
}

#![allow(unused)]
#![warn(clippy::needless_bool_assign)]

fn random() -> bool {
    true
}

fn main() {
    struct Data {
        field: bool,
    };
    let mut a = Data { field: false };
    if random() && random() {
    //~^ ERROR: this if-then-else expression assigns a bool literal
    //~| NOTE: `-D clippy::needless-bool-assign` implied by `-D warnings`
        a.field = true;
    } else {
        a.field = false
    }
    if random() && random() {
    //~^ ERROR: this if-then-else expression assigns a bool literal
        a.field = false;
    } else {
        a.field = true
    }
    // Do not lint…
    if random() {
        a.field = false;
    } else {
        // …to avoid losing this comment
        a.field = true
    }
    // This one also triggers lint `clippy::if_same_then_else`
    // which does not suggest a rewrite.
    if random() {
    //~^ ERROR: this if-then-else expression assigns a bool literal
    //~| ERROR: this `if` has identical blocks
        a.field = true;
    } else {
        a.field = true;
    }
    let mut b = false;
    if random() {
        a.field = false;
    } else {
        b = true;
    }
}

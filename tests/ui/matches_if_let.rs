#![warn(clippy::matches_if_let)]
#![allow(
    clippy::if_same_then_else,
    clippy::needless_ifs,
    clippy::redundant_pattern_matching,
    dead_code,
    unused_variables
)]

enum Enum {
    A,
    B,
    C(u32),
}

macro_rules! wrap_if {
    ($cond:expr) => {
        if $cond {}
    };
}

fn main() {
    let unit = Enum::A;
    if matches!(unit, Enum::A) {}
    //~^ matches_if_let

    let opt = Some(1);
    if false {
    } else if matches!(opt, Some(_)) {
        //~^ matches_if_let
    }

    if matches!(Some(1), Some(_)) {}
    //~^ matches_if_let

    let binding_name = 5;
    let shadowed = Some(1);
    if matches!(shadowed, Some(binding_name)) {
        let _ = binding_name;
    }

    let mixed = Some(1);
    if matches!(mixed, Some(_)) && mixed.is_some() {}

    wrap_if!(matches!(Some(1), Some(_)));

    let mutex = std::sync::Mutex::new(Some(1_u32));
    let ordered = Some(mutex.lock().unwrap());
    if matches!(ordered, Some(_)) {}

    if matches!(mutex.lock().unwrap().as_ref(), Some(_)) {}
}

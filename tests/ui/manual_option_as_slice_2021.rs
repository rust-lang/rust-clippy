//@ edition: 2021
//@ check-pass

#![warn(clippy::manual_option_as_slice)]

fn check(x: Option<u32>) {
    // Edition 2024 drops the `as_ref` temporary before the result of the `if let` is consumed.
    _ = if let Some(ref f) = x.as_ref() {
        std::slice::from_ref(f)
    } else {
        &[]
    };
    _ = if let Some(ref mut f) = x.as_ref() {
        std::slice::from_ref(f)
    } else {
        &[]
    };
}

fn main() {}

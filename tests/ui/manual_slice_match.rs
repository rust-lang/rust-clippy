//@no-rustfix: suggestion is `MaybeIncorrect`
#![warn(clippy::manual_slice_match)]
#![allow(
    clippy::never_loop,
    clippy::redundant_pattern_matching,
    clippy::len_zero,
    clippy::comparison_to_empty,
    unused
)]

fn main() {}

fn vec_is_empty_then_len(v: Vec<u32>) {
    //~v manual_slice_match
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("{}", v[0]);
    } else {
        println!("many");
    }
}

fn slice_len_chain(s: &[u32]) {
    //~v manual_slice_match
    if s.len() == 0 {
        println!("empty");
    } else if s.len() > 1 {
        println!("{} {}", s[0], s[1]);
    } else {
        println!("one");
    }
}

// Should NOT lint: the receiver `s()` has side effects, so collapsing the per-branch
// calls into a single `match s() { .. }` would change how many times `s()` is evaluated.
fn slice_len_chain_impure(s: impl Fn() -> &'static [u32]) {
    if s().len() == 0 {
        println!("empty");
    } else if s().len() > 1 {
        println!("{} {}", s()[0], s()[1]);
    } else {
        println!("one");
    }
}

// Should lint, but with help only: `len() > 100` would need 101 placeholders, which is
// over `max-suggested-slice-pattern-length`, so no concrete `match` is suggested.
fn over_pattern_length_limit(s: &[u32]) {
    //~v manual_slice_match
    if s.is_empty() {
        println!("empty");
    } else if s.len() > 100 {
        println!("large");
    } else {
        println!("small");
    }
}

// Should NOT lint: an array's length is a compile-time constant, so the chain is
// degenerate and a slice pattern of a different length would not type-check.
fn array_len_chain(a: [u32; 4]) {
    if a.len() == 1 {
        println!("one");
    } else if a.len() == 2 {
        println!("two");
    } else {
        println!("other");
    }
}

// Should lint, but with help only: `len() < n` cannot be expressed as a single
// slice pattern, so no concrete `match` is suggested.
fn lt_fallback(v: Vec<u32>) {
    //~v manual_slice_match
    if v.is_empty() {
        println!("empty");
    } else if v.len() < 3 {
        println!("few");
    } else {
        println!("many");
    }
}

// Should NOT lint: third condition is a value guard, not a length check
// (this is the uutils/coreutils#12822 shape).
fn value_guard(files: Vec<String>) {
    if files.is_empty() {
        println!("default");
    } else if files.len() > 1 {
        println!("extra: {}", files[1]);
    } else if files[0] == "-" {
        println!("stdin");
    } else {
        println!("{}", files[0]);
    }
}

// Should NOT lint: conditions inspect different receivers.
fn different_receivers(a: Vec<u32>, b: Vec<u32>) {
    if a.is_empty() {
        println!("a empty");
    } else if b.len() == 1 {
        println!("b one");
    } else {
        println!("other");
    }
}

// Should NOT lint: no final `else`.
fn no_final_else(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("one");
    }
}

// Should NOT lint: only a single condition.
fn single_condition(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else {
        println!("non-empty");
    }
}

// Should NOT lint: `len()` compared against a non-literal.
fn len_against_non_literal(v: Vec<u32>, n: usize) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == n {
        println!("n");
    } else {
        println!("other");
    }
}

// Should NOT lint: slice patterns were stabilised in 1.42.
#[clippy::msrv = "1.41"]
fn below_msrv(v: Vec<u32>) {
    if v.is_empty() {
        println!("empty");
    } else if v.len() == 1 {
        println!("one");
    } else {
        println!("many");
    }
}

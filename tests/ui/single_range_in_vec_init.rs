//@aux-build:proc_macros.rs
//@no-rustfix: overlapping suggestions
#![allow(clippy::temporary_assignment, clippy::no_effect, clippy::useless_vec, unused)]
#![warn(clippy::single_range_in_vec_init)]
#![feature(generic_arg_infer)]

#[macro_use]
extern crate proc_macros;

use std::ops::Range;

macro_rules! a {
    () => {
        vec![0..200];
    };
}

fn awa<T: PartialOrd>(start: T, end: T) {
    [start..end];
}

fn awa_vec<T: PartialOrd>(start: T, end: T) {
    vec![start..end];
}

fn do_not_lint_as_argument(_: Vec<Range<usize>>) {}

fn main() {
    // Lint
    [0..200];
    vec![0..200];
    [0u8..200];
    [0usize..200];
    [0..200usize];
    vec![0u8..200];
    vec![0usize..200];
    vec![0..200usize];
    // Only suggest collect
    [0..200isize];
    vec![0..200isize];
    // Lints, but suggests adding type annotations
    let x = vec![0..200];
    do_not_lint_as_argument(vec![0..200]);
    // Do not lint
    [0..200, 0..100];
    vec![0..200, 0..100];
    [0.0..200.0];
    vec![0.0..200.0];
    // Issue #11086
    let do_not_lint_if_has_type_annotations: Vec<Range<_>> = vec![0..200];
    // https://github.com/rust-lang/rust-clippy/issues/11086#issuecomment-1636996525
    struct DoNotLintStructInitializersUnit(Vec<Range<usize>>);
    struct DoNotLintStructInitializersNamed {
        a: Vec<Range<usize>>,
    };
    enum DoNotLintEnums {
        One(Vec<Range<usize>>),
        Two(Vec<Range<usize>>),
    }
    DoNotLintStructInitializersUnit(vec![0..200]).0 = vec![0..200];
    DoNotLintStructInitializersNamed { a: vec![0..200] }.a = vec![0..200];
    let o = DoNotLintEnums::One(vec![0..200]);
    match o {
        DoNotLintEnums::One(mut e) => e = vec![0..200],
        _ => todo!(),
    }
    do_not_lint_as_argument(vec![0..200]);
    // `Copy` is not implemented for `Range`, so this doesn't matter
    // FIXME: [0..200; 2];
    // FIXME: [vec!0..200; 2];

    // Unfortunately skips any macros
    a!();

    // Skip external macros and procedural macros
    external! {
        [0..200];
        vec![0..200];
    }
    with_span! {
        span
        [0..200];
        vec![0..200];
    }
}

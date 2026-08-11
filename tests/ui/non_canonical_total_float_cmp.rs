//@aux-build:proc_macros.rs
#![feature(f16)]
#![feature(f128)]
#![warn(clippy::non_canonical_total_float_cmp)]
use std::cmp;
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};

extern crate proc_macros;

fn example() {
    93f32.partial_cmp(&83f32).unwrap_or(Ordering::Less);
    //~^ non_canonical_total_float_cmp

    93.0f16.partial_cmp(&83.0).unwrap_or(Ordering::Greater);
    //~^ non_canonical_total_float_cmp

    93.0f128.partial_cmp(&83.0).unwrap_or(Ordering::Equal);
    //~^ non_canonical_total_float_cmp

    93.0f64.partial_cmp(&83.0).unwrap_or(Less);
    //~^ non_canonical_total_float_cmp

    93.0_f32.partial_cmp(&83.0).unwrap_or(Greater);
    //~^ non_canonical_total_float_cmp

    93f64.partial_cmp(&83f64).unwrap_or(Equal);
    //~^ non_canonical_total_float_cmp

    let x = cmp::Ordering::Less;
    93f32.partial_cmp(&83f32).unwrap_or(x);
    //~^ non_canonical_total_float_cmp

    let x: f32 = 0.0;
    x.partial_cmp(&83f32).unwrap_or(core::cmp::Ordering::Greater);
    //~^ non_canonical_total_float_cmp

    let x: f32 = 0.0;
    93f32.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal);
    //~^ non_canonical_total_float_cmp

    //
    // Tests for tokens that come from expansion.
    //

    // no lint: can't change to "total_cmp"
    proc_macros::external!(93f32.partial_cmp(&83f32).unwrap_or(Equal));

    // Ideally, this would lint, but that requires a bigger chance to the
    // way that method lints are handled.
    proc_macros::external!(93f32).partial_cmp(&83f32).unwrap_or(Equal);

    // Ideally, this would lint, but that requires a bigger chance to the
    // way that method lints are handled.
    93f32.partial_cmp(proc_macros::external!(&83f32)).unwrap_or(Equal);

    // no lint: can't change to "total_cmp"
    proc_macros::external!(93f32.partial_cmp(&83f32)).unwrap_or(Equal);

    // Ideally, this would lint, but that requires a bigger chance to the
    // way that method lints are handled.
    93f32.partial_cmp(&83f32).unwrap_or(proc_macros::external!(Equal));
}

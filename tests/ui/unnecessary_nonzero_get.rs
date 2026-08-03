#![warn(clippy::unnecessary_nonzero_get)]
// the `msrv` functions below deliberately use items newer than their MSRV
#![allow(clippy::incompatible_msrv)]

use std::num::{NonZero, NonZeroI32, NonZeroU32};

fn unsigned(nz: NonZero<u32>) {
    let _ = nz.get().leading_zeros();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().trailing_zeros();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().is_power_of_two();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().ilog2();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().ilog10();
    //~^ unnecessary_nonzero_get
}

fn signed(nz: NonZero<i32>) {
    let _ = nz.get().leading_zeros();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().trailing_zeros();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().is_positive();
    //~^ unnecessary_nonzero_get
    let _ = nz.get().is_negative();
    //~^ unnecessary_nonzero_get
}

fn unsigned_widths(a: NonZero<u8>, b: NonZero<u16>, c: NonZero<u64>, d: NonZero<u128>, e: NonZero<usize>) {
    let _ = a.get().leading_zeros();
    //~^ unnecessary_nonzero_get
    let _ = b.get().trailing_zeros();
    //~^ unnecessary_nonzero_get
    let _ = c.get().ilog2();
    //~^ unnecessary_nonzero_get
    let _ = d.get().is_power_of_two();
    //~^ unnecessary_nonzero_get
    let _ = e.get().ilog10();
    //~^ unnecessary_nonzero_get
}

fn signed_widths(f: NonZero<i8>, g: NonZero<i16>, h: NonZero<i64>, i: NonZero<i128>, j: NonZero<isize>) {
    let _ = f.get().is_negative();
    //~^ unnecessary_nonzero_get
    let _ = g.get().is_positive();
    //~^ unnecessary_nonzero_get
    let _ = h.get().leading_zeros();
    //~^ unnecessary_nonzero_get
    let _ = i.get().trailing_zeros();
    //~^ unnecessary_nonzero_get
    let _ = j.get().leading_zeros();
    //~^ unnecessary_nonzero_get
}

fn aliases_and_receivers(a: NonZeroU32, b: NonZeroI32, r: &NonZero<u32>) {
    let _ = a.get().ilog2();
    //~^ unnecessary_nonzero_get
    let _ = b.get().is_positive();
    //~^ unnecessary_nonzero_get
    let _ = r.get().leading_zeros();
    //~^ unnecessary_nonzero_get

    // more complex receiver expressions
    let _ = NonZero::new(5u32).unwrap().get().leading_zeros();
    //~^ unnecessary_nonzero_get
    let _ = (a).get().trailing_zeros();
    //~^ unnecessary_nonzero_get

    // multi-line chain
    let _ = a
        .get()
        //~^ unnecessary_nonzero_get
        .leading_zeros();
}

fn no_lint(nz: NonZero<u32>, signed: NonZero<i32>, plain: u32) {
    // `NonZero`'s version returns `NonZero<u32>` rather than `u32`, so the `get` would only move
    // rather than disappear
    let _ = nz.get().bit_width();
    let _ = nz.get().count_ones();
    let _ = nz.get().isqrt();
    let _ = nz.get().checked_add(1);
    let _ = nz.get().saturating_mul(2);
    let _ = signed.get().abs();
    let _ = signed.get().cast_unsigned();
    let _ = signed.get().unsigned_abs();
    let _ = signed.get().wrapping_neg();
    let _ = signed.get().overflowing_neg();

    // `highest_one`/`lowest_one` return `Option<u32>` on integers but `u32` on `NonZero`
    let _ = nz.get().highest_one();
    let _ = nz.get().lowest_one();

    // no `NonZero` equivalent at all
    let _ = nz.get().to_string();
    let _ = nz.get().count_zeros();

    // `i32::ilog2` exists but `NonZero<i32>::ilog2` does not, so the impls must not cross over
    let _ = signed.get().ilog2();

    // not a `NonZero` receiver
    let _ = plain.leading_zeros();

    // `get` is not immediately followed by the method
    let x = nz.get();
    let _ = x.leading_zeros();

    // takes arguments
    let _ = nz.get().rotate_left(2);

    // a shadowing trait method must not be rewritten
    let _ = nz.get().shadowed();
}

trait Shadowed {
    fn shadowed(self) -> u32;
}

impl Shadowed for u32 {
    fn shadowed(self) -> u32 {
        self
    }
}

impl Shadowed for NonZero<u32> {
    fn shadowed(self) -> u32 {
        0
    }
}

// `NonZero::leading_zeros` was stabilized in 1.53.0
#[clippy::msrv = "1.52"]
fn below_msrv(nz: NonZero<u32>) {
    let _ = nz.get().leading_zeros();
}

#[clippy::msrv = "1.53"]
fn meets_msrv(nz: NonZero<u32>) {
    let _ = nz.get().leading_zeros();
    //~^ unnecessary_nonzero_get
}

// `NonZero::ilog2` was stabilized in 1.67.0, later than `leading_zeros`
#[clippy::msrv = "1.66"]
fn below_ilog2_msrv(nz: NonZero<u32>) {
    let _ = nz.get().ilog2();
    let _ = nz.get().leading_zeros();
    //~^ unnecessary_nonzero_get
}

fn main() {}

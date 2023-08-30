#![allow(unused)]

#[derive(Debug)]
struct Foo;

const VAR_ONE: &'static str = "Test constant #1"; // ERROR: Consider removing 'static.
//~^ ERROR: constants have by default a `'static` lifetime
//~| NOTE: `-D clippy::redundant-static-lifetimes` implied by `-D warnings`

const VAR_TWO: &str = "Test constant #2"; // This line should not raise a warning.

const VAR_THREE: &[&'static str] = &["one", "two"]; // ERROR: Consider removing 'static
//~^ ERROR: constants have by default a `'static` lifetime

const VAR_FOUR: (&str, (&str, &'static str), &'static str) = ("on", ("th", "th"), "on"); // ERROR: Consider removing 'static
//~^ ERROR: constants have by default a `'static` lifetime
//~| ERROR: constants have by default a `'static` lifetime

const VAR_SIX: &'static u8 = &5;
//~^ ERROR: constants have by default a `'static` lifetime

const VAR_HEIGHT: &'static Foo = &Foo {};
//~^ ERROR: constants have by default a `'static` lifetime

const VAR_SLICE: &'static [u8] = b"Test constant #1"; // ERROR: Consider removing 'static.
//~^ ERROR: constants have by default a `'static` lifetime

const VAR_TUPLE: &'static (u8, u8) = &(1, 2); // ERROR: Consider removing 'static.
//~^ ERROR: constants have by default a `'static` lifetime

const VAR_ARRAY: &'static [u8; 1] = b"T"; // ERROR: Consider removing 'static.
//~^ ERROR: constants have by default a `'static` lifetime

static STATIC_VAR_ONE: &'static str = "Test static #1"; // ERROR: Consider removing 'static.
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_TWO: &str = "Test static #2"; // This line should not raise a warning.

static STATIC_VAR_THREE: &[&'static str] = &["one", "two"]; // ERROR: Consider removing 'static
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_SIX: &'static u8 = &5;
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_HEIGHT: &'static Foo = &Foo {};
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_SLICE: &'static [u8] = b"Test static #3"; // ERROR: Consider removing 'static.
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_TUPLE: &'static (u8, u8) = &(1, 2); // ERROR: Consider removing 'static.
//~^ ERROR: statics have by default a `'static` lifetime

static STATIC_VAR_ARRAY: &'static [u8; 1] = b"T"; // ERROR: Consider removing 'static.
//~^ ERROR: statics have by default a `'static` lifetime

static mut STATIC_MUT_SLICE: &'static mut [u32] = &mut [0];
//~^ ERROR: statics have by default a `'static` lifetime

fn main() {
    let false_positive: &'static str = "test";

    unsafe {
        STATIC_MUT_SLICE[0] = 0;
    }
}

trait Bar {
    const TRAIT_VAR: &'static str;
}

impl Foo {
    const IMPL_VAR: &'static str = "var";
}

impl Bar for Foo {
    const TRAIT_VAR: &'static str = "foo";
}

#[clippy::msrv = "1.16"]
fn msrv_1_16() {
    static V: &'static u8 = &16;
}

#[clippy::msrv = "1.17"]
fn msrv_1_17() {
    static V: &'static u8 = &17;
    //~^ ERROR: statics have by default a `'static` lifetime
}

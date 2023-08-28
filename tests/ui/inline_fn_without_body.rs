#![warn(clippy::inline_fn_without_body)]
#![allow(clippy::inline_always)]

trait Foo {
    #[inline]
    //~^ ERROR: use of `#[inline]` on trait method `default_inline` which has no body
    //~| NOTE: `-D clippy::inline-fn-without-body` implied by `-D warnings`
    fn default_inline();

    #[inline(always)]
    //~^ ERROR: use of `#[inline]` on trait method `always_inline` which has no body
    fn always_inline();

    #[inline(never)]
    //~^ ERROR: use of `#[inline]` on trait method `never_inline` which has no body
    fn never_inline();

    #[inline]
    fn has_body() {}
}

fn main() {}

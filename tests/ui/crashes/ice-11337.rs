#![feature(trait_alias)]
#![allow(clippy::toilet_closures)]

trait Confusing<F> = Fn(i32) where F: Fn(u32);

fn alias<T: Confusing<F>, F>(_: T, _: F) {}

fn main() {
    alias(|_| {}, |_| {});
}

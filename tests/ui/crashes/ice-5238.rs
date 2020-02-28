// Regression test for #5238

#![feature(generators, generator_trait)]

fn main() {
    let _ = || {
        yield;
    };
}

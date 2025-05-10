#![warn(clippy::range_minus_one, clippy::range_plus_one)]
#![allow(unused_parens)]
#![allow(clippy::iter_with_drain)]
fn f() -> usize {
    42
}

macro_rules! macro_plus_one {
    ($m: literal) => {
        for i in 0..$m + 1 {
            println!("{}", i);
        }
    };
}

macro_rules! macro_minus_one {
    ($m: literal) => {
        for i in 0..=$m - 1 {
            println!("{}", i);
        }
    };
}

fn main() {
    for _ in 0..2 {}
    for _ in 0..=2 {}

    for _ in 0..3 + 1 {}
    //~^ range_plus_one
    for _ in 0..=3 + 1 {}

    for _ in 0..1 + 5 {}
    //~^ range_plus_one
    for _ in 0..=1 + 5 {}

    for _ in 1..1 + 1 {}
    //~^ range_plus_one
    for _ in 1..=1 + 1 {}

    for _ in 0..13 + 13 {}
    for _ in 0..=13 - 7 {}

    for _ in 0..(1 + f()) {}
    //~^ range_plus_one
    for _ in 0..=(1 + f()) {}

    // Those are not linted, as in the general case we cannot be sure that the exact type won't be
    // important.
    let _ = ..11 - 1;
    let _ = ..=11 - 1;
    let _ = ..=(11 - 1);
    let _ = (1..11 + 1);
    let _ = (f() + 1)..(f() + 1);

    const ONE: usize = 1;
    // integer consts are linted, too
    for _ in 1..ONE + ONE {}
    //~^ range_plus_one

    let mut vec: Vec<()> = std::vec::Vec::new();
    vec.drain(..);

    macro_plus_one!(5);
    macro_minus_one!(5);

    // As an instance of `Iterator`
    (1..10 + 1).for_each(|_| {});
    //~^ range_plus_one

    // As an instance of `IntoIterator`
    #[allow(clippy::useless_conversion)]
    (1..10 + 1).into_iter().for_each(|_| {});
    //~^ range_plus_one

    // As an instance of `RangeBounds`
    {
        use std::ops::RangeBounds;
        let _ = (1..10 + 1).start_bound();
        //~^ range_plus_one
    }

    // As a `SliceIndex`
    let a = [10, 20, 30];
    let _ = &a[1..1 + 1];
    //~^ range_plus_one

    // As method call argument
    vec.drain(2..3 + 1);
    //~^ range_plus_one

    // As function call argument
    take_arg(10..20 + 1);
    //~^ range_plus_one

    // Do not lint, as the same type is used for both parameters
    take_args(10..20 + 1, 10..21);
}

fn take_arg<T: Iterator<Item = u32>>(_: T) {}
fn take_args<T: Iterator<Item = u32>>(_: T, _: T) {}

fn issue9908() {
    // Simplified test case
    let _ = || 0..=1;

    // Original test case
    let full_length = 1024;
    let range = {
        // do some stuff, omit here
        None
    };

    let range = range.map(|(s, t)| s..=t).unwrap_or(0..=(full_length - 1));

    assert_eq!(range, 0..=1023);
}

fn issue9908_2(n: usize) -> usize {
    (1..=n - 1).sum()
    //~^ range_minus_one
}

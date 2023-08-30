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

#[warn(clippy::range_plus_one)]
#[warn(clippy::range_minus_one)]
fn main() {
    for _ in 0..2 {}
    for _ in 0..=2 {}

    for _ in 0..3 + 1 {}
    //~^ ERROR: an inclusive range would be more readable
    //~| NOTE: `-D clippy::range-plus-one` implied by `-D warnings`
    for _ in 0..=3 + 1 {}

    for _ in 0..1 + 5 {}
    //~^ ERROR: an inclusive range would be more readable
    for _ in 0..=1 + 5 {}

    for _ in 1..1 + 1 {}
    //~^ ERROR: an inclusive range would be more readable
    for _ in 1..=1 + 1 {}

    for _ in 0..13 + 13 {}
    for _ in 0..=13 - 7 {}

    for _ in 0..(1 + f()) {}
    //~^ ERROR: an inclusive range would be more readable
    for _ in 0..=(1 + f()) {}

    let _ = ..11 - 1;
    let _ = ..=11 - 1;
    //~^ ERROR: an exclusive range would be more readable
    //~| NOTE: `-D clippy::range-minus-one` implied by `-D warnings`
    let _ = ..=(11 - 1);
    //~^ ERROR: an exclusive range would be more readable
    let _ = (1..11 + 1);
    //~^ ERROR: an inclusive range would be more readable
    let _ = (f() + 1)..(f() + 1);
    //~^ ERROR: an inclusive range would be more readable

    const ONE: usize = 1;
    // integer consts are linted, too
    for _ in 1..ONE + ONE {}
    //~^ ERROR: an inclusive range would be more readable

    let mut vec: Vec<()> = std::vec::Vec::new();
    vec.drain(..);

    macro_plus_one!(5);
    macro_minus_one!(5);
}

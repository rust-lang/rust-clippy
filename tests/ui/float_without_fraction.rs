//@no-rustfix: overlapping suggestions
#![warn(clippy::float_without_fraction)]

fn main() {
    let _ = 0.;
    //~^ float_without_fraction
    let _ = 1234.;
    //~^ float_without_fraction
    let _ = 0_.;
    //~^ float_without_fraction
    //~| inconsistent_digit_grouping
}

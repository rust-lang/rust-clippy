//@no-rustfix
#![warn(clippy::implicit_saturating_sub)]

fn main() {
    let a = 12u32;
    let b = 13u32;

    let result = if a > b { b - a } else { 0 };
    //~^ ERROR: inverted arithmetic check before subtraction
    let result = if b < a { b - a } else { 0 };
    //~^ ERROR: inverted arithmetic check before subtraction

    let result = if a > b { 0 } else { a - b };
    //~^ ERROR: inverted arithmetic check before subtraction
    let result = if b < a { 0 } else { a - b };
    //~^ ERROR: inverted arithmetic check before subtraction
}

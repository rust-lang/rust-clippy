#![warn(clippy::large_const_arrays)]
//@no-rustfix
const ABOVE: [u8; 11] = [0; 11];
//~^ large_const_arrays
const BELOW: [u8; 10] = [0; 10];

fn main() {}

#![warn(clippy::large_stack_arrays)]

fn main() {
    let above = [0u8; 11];
    //~^ large_stack_arrays
    let below = [0u8; 10];
}

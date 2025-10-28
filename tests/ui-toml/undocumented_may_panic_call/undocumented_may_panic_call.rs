#![warn(clippy::undocumented_may_panic_call)]

fn main() {
    let mut v = vec![1, 2, 3];

    v.push(4);
    //~^ undocumented_may_panic_call

    // Panic: The capaticy is less than isize::MAX, so we are safe!
    v.push(5);
}

#![warn(clippy::repeat_once)]
#[allow(unused, clippy::redundant_clone)]
fn main() {
    const N: usize = 1;
    let s = "str";
    let string = "String".to_string();
    let slice = [1; 5];

    let a = [1; 5].repeat(1);
    //~^ ERROR: calling `repeat(1)` on slice
    //~| NOTE: `-D clippy::repeat-once` implied by `-D warnings`
    let b = slice.repeat(1);
    //~^ ERROR: calling `repeat(1)` on slice
    let c = "hello".repeat(N);
    //~^ ERROR: calling `repeat(1)` on str
    let d = "hi".repeat(1);
    //~^ ERROR: calling `repeat(1)` on str
    let e = s.repeat(1);
    //~^ ERROR: calling `repeat(1)` on str
    let f = string.repeat(1);
    //~^ ERROR: calling `repeat(1)` on a string literal
}

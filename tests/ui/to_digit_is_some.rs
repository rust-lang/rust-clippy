#![warn(clippy::to_digit_is_some)]

fn main() {
    let c = 'x';
    let d = &c;

    let _ = d.to_digit(8).is_some();
    //~^ ERROR: use of `.to_digit(..).is_some()`
    //~| NOTE: `-D clippy::to-digit-is-some` implied by `-D warnings`
    let _ = char::to_digit(c, 8).is_some();
    //~^ ERROR: use of `.to_digit(..).is_some()`
}

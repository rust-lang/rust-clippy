#![warn(clippy::is_digit_ascii_radix)]

const TEN: u32 = 10;

fn main() {
    let c: char = '6';

    // Should trigger the lint.
    let _ = c.is_digit(10);
    //~^ ERROR: use of `char::is_digit` with literal radix of 10
    //~| NOTE: `-D clippy::is-digit-ascii-radix` implied by `-D warnings`
    let _ = c.is_digit(16);
    //~^ ERROR: use of `char::is_digit` with literal radix of 16
    let _ = c.is_digit(0x10);
    //~^ ERROR: use of `char::is_digit` with literal radix of 16

    // Should not trigger the lint.
    let _ = c.is_digit(11);
    let _ = c.is_digit(TEN);
}

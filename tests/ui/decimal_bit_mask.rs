#![allow(clippy::no_effect)]
#![warn(clippy::decimal_bit_mask)]
fn main() {
    let mut x = 0;
    // BAD: Bitwise operation, decimal literal, one literal
    x & 99; //~ decimal_bit_mask
    x | 99; //~ decimal_bit_mask
    x ^ 99; //~ decimal_bit_mask
    x &= 99; //~ decimal_bit_mask
    x |= 99; //~ decimal_bit_mask
    x ^= 99; //~ decimal_bit_mask

    // BAD: Bitwise operation, decimal literal, two literals
    0b1010 & 99; //~ decimal_bit_mask
    0b1010 | 99; //~ decimal_bit_mask
    0b1010 ^ 99; //~ decimal_bit_mask
    99 & 0b1010; //~ decimal_bit_mask
    99 | 0b1010; //~ decimal_bit_mask
    99 ^ 0b1010; //~ decimal_bit_mask
    0xD | 99; //~ decimal_bit_mask
    88 & 99; //~ decimal_bit_mask

    // GOOD: Bitwise operation, binary/hex literal, one literal
    x & 0b1010;
    x | 0b1010;
    x ^ 0b1010;
    x &= 0b1010;
    x |= 0b1010;
    x ^= 0b1010;
    x & 0xD;

    // GOOD: Bitwise operation, binary/hex literal, two literals
    0b1010 & 0b1101;
    0xD ^ 0xF;

    // GOOD: Numeric operations, any literal
    x += 99;
    x -= 0b1010;
    x *= 0xD;
    99 + 99;
    0b1010 - 0b1101;
    0xD * 0xD;

    // GOOD: Bitwise operations, variables only
    let y = 0;
    x & y;
    x &= y;
    x + y;
    x += y;
}

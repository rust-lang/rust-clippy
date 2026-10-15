#![warn(clippy::redundant_modulo_divisor)]
#![allow(clippy::no_effect, clippy::unnecessary_operation)]

const DIVISOR: u32 = 8;

fn rem_literals() -> u32 {
    (1_u32 + 8 + 2) % 8
    //~^ redundant_modulo_divisor
}

fn rem_const_divisor() -> u32 {
    (1_u32 + DIVISOR + 2) % DIVISOR
    //~^ redundant_modulo_divisor
}

fn rem_grouped_expression_divisor() -> u32 {
    (1_u32 + (4 + 4) + 2) % (4 + 4)
    //~^ redundant_modulo_divisor
}

// Signed remainder is not equivalent when removing the divisor changes the
// sign of the dividend: `(-1 + 8) % 8` is 7, while `-1 % 8` is -1.
fn signed_remainder_is_not_linted() -> i32 {
    (-1_i32 + 8 + 2) % 8
}

// The lint cannot prove that arbitrary operands avoid overflow.
fn unknown_unsigned_operands_are_not_linted(x: u32, y: u32) -> u32 {
    (x + 8 + y) % 8
}

// Even unsigned wrapping addition can change the remainder for a divisor
// that does not divide the type's wraparound modulus.
fn unsigned_overflow_is_not_linted() -> u8 {
    (250_u8 + 10 + 3) % 10
}

// In checked builds, removing this overflowing addition would also remove a
// panic, so overflowing constant expressions are not linted either.
fn overflowing_constant_is_not_linted() -> u8 {
    (250_u8 + 8 + 1) % 8
}

fn main() {}

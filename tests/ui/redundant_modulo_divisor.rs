#![warn(clippy::redundant_modulo_divisor)]
#![allow(clippy::no_effect, clippy::unnecessary_operation)]

fn rem_u32(x: u32, y: u32, n: u32) -> u32 {
    (x + n + y) % n
    //~^ redundant_modulo_divisor
}

fn rem_i32(x: i32, y: i32, n: i32) -> i32 {
    (x + n + y) % n
    //~^ redundant_modulo_divisor
}

fn divisor_inside_grouped_rhs_addition_is_ignored(x: u32, y: u32, n: u32) -> u32 {
    (x + (n + y)) % n
}

fn rem_trailing_divisor(x: u32, y: u32, m: u32) -> u32 {
    (x + y + m) % m
    //~^ redundant_modulo_divisor
}

fn rem_leading_divisor(x: u32, y: u32, n: u32) -> u32 {
    (n + x + y) % n
    //~^ redundant_modulo_divisor
}

fn divisor_inside_grouped_trailing_addition_is_ignored(x: u32, y: u32, n: u32) -> u32 {
    (x + (y + n)) % n
}

fn rem_parenthesized_rhs(x: u32, y: u32, n: u32) -> u32 {
    (x + (n) + y) % (n)
    //~^ redundant_modulo_divisor
}

fn rem_repeated_divisor(x: u32, n: u32) -> u32 {
    (n + x + n) % n
    //~^ redundant_modulo_divisor
}

fn rem_grouped_expression_divisor(x: usize, y: usize, o: usize) -> usize {
    (x + (o + o) + y) % (o + o)
    //~^ redundant_modulo_divisor
}

fn rem_nested_expression_divisor(x: u32, y: u32, n: u32, z: u32) -> u32 {
    (x + (n + y) + z) % (n + y)
    //~^ redundant_modulo_divisor
}

fn rem_with_literal_term(x: u32, y: u32, n: u32) -> u32 {
    (x + n + 1 + y) % n
    //~^ redundant_modulo_divisor
}

fn no_divisor_term(x: u32, y: u32, n: u32) -> u32 {
    (x + y) % n
}

fn two_terms_with_divisor(x: u32, n: u32) -> u32 {
    (x + n) % n
}

fn subtraction_u32(x: u32, y: u32, n: u32) -> u32 {
    (x + n - y) % n
}

fn nested_subtraction_u32(x: u32, y: u32, n: u32) -> u32 {
    (x + (n - y)) % n
}

fn multiplication_u32(x: u32, y: u32, n: u32) -> u32 {
    (x + n * y) % n
}

fn division_u32(x: u32, y: u32, n: u32) -> u32 {
    (x + n + y) / n
}

fn rem_assign_u32(x: u32, y: u32, n: u32) -> u32 {
    let mut rem = x + n + y;
    rem %= n;
    rem
}

fn expression_divisor_not_present(x: u32, y: u32, n: u32, z: u32) -> u32 {
    (x + (n + z) + y) % (n + y)
}

fn add_subterm_with_literal(x: u32, y: u32, n: u32) -> u32 {
    (x + (n + 1) + y) % n
}

fn expression_divisor_not_contiguous(x: usize, y: usize, o: usize) -> usize {
    (x + (o + o) + y + o) % (o + o + o)
}

fn not_integer(x: f32, y: f32, n: f32) -> f32 {
    (x + n + y) % n
}

fn main() {}

#![warn(clippy::large_digit_groups)]

fn main() {
    macro_rules! mac {
        () => {
            0b1_10110_i64
        };
    }

    let _good = (
        0b1011_i64,
        0o1_234_u32,
        0x1_234_567,
        1_2345_6789,
        1234_f32,
        1_234.12_f32,
        1_234.123_f32,
        1.123_4_f32,
    );
    let _bad = (
        0b1_10110_i64,
        0xd_e_adbee_f_usize,
        //~^ ERROR: digits of hex, binary or octal literal not in groups of equal size
        //~| NOTE: `-D clippy::unusual-byte-groupings` implied by `-D warnings`
        1_23456_f32,
        //~^ ERROR: digit groups should be smaller
        //~| NOTE: `-D clippy::large-digit-groups` implied by `-D warnings`
        1_23456.12_f32,
        //~^ ERROR: digit groups should be smaller
        1_23456.12345_f64,
        //~^ ERROR: digit groups should be smaller
        1_23456.12345_6_f64,
        //~^ ERROR: digit groups should be smaller
    );
    // Ignore literals in macros
    let _ = mac!();
}

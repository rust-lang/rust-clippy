use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::numeric_literal::{IntStr, Radix};
use clippy_utils::source::SpanExt as _;
use core::mem::transmute;
use core::simd::Simd;
use core::simd::cmp::{SimdPartialEq as _, SimdPartialOrd as _};
use core::simd::num::SimdInt as _;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext as _};
use rustc_span::Span;

use super::DECIMAL_LITERAL_REPRESENTATION;

pub(super) fn check(cx: &EarlyContext<'_>, threshold: u64, num: IntStr<'_>, sp: Span) {
    if let Radix::Dec = num.radix
        && num.suffix.is_none_or(|s| !s.is_float())
        && let Some(val @ 1..) = num.parse_as_u128()
        && val >= u128::from(threshold)
        && has_simple_hex_repr(val)
        && let sp_data = sp.data()
        && !sp_data.ctxt.in_external_macro(cx.sess().source_map())
        // Proc-macro detection.
        && sp_data.check_text(cx, |src| num.eq_str(src))
    {
        span_lint_and_then(
            cx,
            DECIMAL_LITERAL_REPRESENTATION,
            sp,
            "integer literal has a better hexadecimal representation",
            |diag| {
                diag.span_suggestion_verbose(
                    num.trim_sp_to_digits(&sp_data),
                    "use a hex literal",
                    fmt(val),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

fn has_simple_hex_repr(val: u128) -> bool {
    // Power-of-two or power-of-two minus one.
    // Ignore the lowest digit for larger numbers.
    let pval = if val > 0xfff { val >> 4 } else { val };
    if pval.is_power_of_two() || pval.wrapping_add(1).count_ones() <= 1 {
        return true;
    }

    // Check if all digits are `0` or ` F`. Optionally allow the most significant
    // non-zero digit to be `7`.

    // Separate all the digits (lsb first)
    let bytes = Simd::from_array(val.to_le_bytes());
    let mask = Simd::splat(0xf);
    let (low, high) = Simd::interleave(bytes & mask, bytes >> 4);

    // Get bitmasks for the `0`, `7` and `F` digits.
    let f_digits: u32 = (low.simd_eq(mask).to_bitmask() | (high.simd_eq(mask).to_bitmask() << 16)).truncate();
    let zeros = Simd::splat(0);
    let zero_digits: u32 = (low.simd_eq(zeros).to_bitmask() | (high.simd_eq(zeros).to_bitmask() << 16)).truncate();
    let sevens = Simd::splat(7);
    let seven_digits: u32 = (low.simd_eq(sevens).to_bitmask() | (high.simd_eq(sevens).to_bitmask() << 16)).truncate();

    // Get the bit for the least significant digit which is neither `0` nor `F`.
    let least_non_zero_f_bit = (zero_digits | f_digits).wrapping_add(1);
    let changed_mask = least_non_zero_f_bit ^ (zero_digits | f_digits);
    let least_non_zero_f_bit = changed_mask & least_non_zero_f_bit;

    // Check if the isolated bit (if any) matches the combined set of `7` digits
    // and the upper non-zero bits. e.g.
    let upper_non_zero_mask = !changed_mask & !zero_digits;
    #[expect(clippy::needless_return)]
    return least_non_zero_f_bit == seven_digits | upper_non_zero_mask;

    // Examples (if the masks were u8):
    // value : 0xff00ff00, 0x7f00ff00, 0x007f0000, 0x00710000, 0xf7000000
    // 0     : 0b00110011, 0b00110011, 0b11001111, 0b11001111, 0x00111111
    // 7     : 0b00000000, 0b10000000, 0b00100000, 0b00100000, 0x01000000
    // F     : 0b11001100, 0b01001100, 0b00010000, 0b00000000, 0x10000000
    // 0|F   : 0b11111111, 0b01111111, 0b11011111, 0b11001111, 0x10111111
    // +1    : 0b00000000, 0b10000000, 0b11100000, 0b11010000, 0b11000000
    // change: 0b11111111, 0b11111111, 0b00111111, 0b00011111, 0b01111111
    // least : 0b00000000, 0b10000000, 0b00100000, 0b00010000, 0b01000000
    // upper : 0b00000000, 0b00000000, 0b00000000, 0b00100000, 0b10000000
    // cmp   : 0b00000000, 0b10000000, 0b00100000, 0b00100000, 0b11000000
    // lint  :        yes,        yes,        yes,         no,         no
}

fn fmt(val: u128) -> String {
    let bytes = Simd::from_array(val.to_be_bytes());
    let (high, low) = Simd::interleave(bytes >> 4, bytes & Simd::splat(15));

    // Get the index of the first non-zero digit (msb first).
    let high_zero_mask = high.simd_eq(Simd::splat(0)).to_bitmask();
    let low_zero_mask = low.simd_eq(Simd::splat(0)).to_bitmask();
    let non_zero_mask: u32 = (!(high_zero_mask | low_zero_mask << 16)).truncate();
    let first_digit = (non_zero_mask | (1 << 31)).trailing_zeros();

    // Get the character for each digit and group them into u32 groups (msb first).
    let high_is_alpha: Simd<u8, _> = high.simd_gt(Simd::splat(9)).to_simd().cast();
    let low_is_alpha: Simd<u8, _> = low.simd_gt(Simd::splat(9)).to_simd().cast();
    let high = high + Simd::splat(b'0') + (Simd::splat(b'A' - b'0' - 10) & high_is_alpha);
    let low = low + Simd::splat(b'0') + (Simd::splat(b'A' - b'0' - 10) & low_is_alpha);
    let mut digits = [0u8; 32];
    digits[..16].copy_from_slice(&high.to_array());
    digits[16..].copy_from_slice(&low.to_array());
    // SAFETY: trivial integer array conversion
    let digits = unsafe { transmute::<[u8; 32], [u32; 8]>(digits) };

    let mut buf = [0u8; 32 + 32 / 4 + 1];
    let mut i = 0;
    buf[i..i + 2].copy_from_slice(b"0x");
    i += 2;

    let first_group = first_digit as usize / 4;
    let group = u32::from_le_bytes(digits[first_group].to_ne_bytes()) >> (first_digit % 4 * 8);
    buf[i..i + 4].copy_from_slice(&group.to_le_bytes());
    i += 4 - (first_digit as usize % 4);
    for &group in &digits[first_group + 1..] {
        buf[i] = b'_';
        buf[i + 1..i + 5].copy_from_slice(&group.to_ne_bytes());
        i += 5;
    }

    String::from_utf8(buf[..i].to_vec()).unwrap_or_default()
}

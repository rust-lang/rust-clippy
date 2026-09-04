use arrayvec::ArrayVec;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::numeric_literal::{IntStr, Radix};
use clippy_utils::source::SpanExt;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext};
use rustc_span::Span;

use super::DECIMAL_LITERAL_REPRESENTATION;

pub(super) fn check(cx: &EarlyContext<'_>, threshold: u64, num: IntStr<'_>, sp: Span) {
    if let Radix::Dec = num.radix
        && num.suffix.is_none_or(|s| !s.is_float())
        && let Some(val @ 1..) = num.parse_as_u128()
        && val >= u128::from(threshold)
        && check_val(val)
        && let sp_data = sp.data()
        && !sp_data.ctxt.in_external_macro(cx.sess().source_map())
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

fn check_val(mut val: u128) -> bool {
    // Power-of-two or power-of-two minus one.
    // Ignore the lowest digit for larger numbers.
    let pval = if val > 0xfff { val >> 4 } else { val };
    if pval.count_ones() == 1 || pval.wrapping_add(1).count_ones() <= 1 {
        return true;
    }
    // All hex digits are either `0` or `f`.
    for _ in 0..15 {
        if !matches!(val & 0xff, 0 | 0xf | 0xf0 | 0xff) {
            break;
        }
        val >>= 8;
    }
    matches!(val, 0 | 0x7 | 0xf | 0x70 | 0x7f | 0xf0 | 0xff)
}

fn fmt(mut num: u128) -> String {
    let mut buf = ArrayVec::<u8, { 128 / 16 + 128 / 16 / 4 + 2 }>::new();
    let mut i = 4u8;
    while num != 0 {
        if i == 0 {
            let _ = buf.try_push(b'_');
            i = 3;
        } else {
            i -= 1;
        }
        let _ = buf.try_push(match num % 16 {
            c @ 0..10 => c as u8 + b'0',
            c => c as u8 - 10 + b'a',
        });
        num /= 16;
    }
    let _ = buf.try_extend_from_slice(b"x0");
    buf.reverse();
    String::from_utf8(buf.to_vec()).unwrap_or(String::new())
}

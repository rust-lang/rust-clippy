use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::numeric_literal::{FloatStr, IntStr};
use clippy_utils::source::SpanExt;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext};
use rustc_span::{BytePos, Span};

use super::MISTYPED_LITERAL_SUFFIXES;

/// Returns `true` if the lint is emitted.
pub(super) fn check_int(cx: &EarlyContext<'_>, num: IntStr<'_>, sp: Span) -> bool {
    let (text, max) = match num.text.as_bytes() {
        [.., b'_', b'8'] => (&num.text[..num.text.len() - 2], u64::from(u8::MAX)),
        [.., b'_', b'1', b'6'] => (&num.text[..num.text.len() - 3], u64::from(u16::MAX)),
        [.., b'_', b'3', b'2'] => (&num.text[..num.text.len() - 3], u64::from(u32::MAX)),
        [.., b'_', b'6', b'4'] => (&num.text[..num.text.len() - 3], u64::MAX),
        _ => return false,
    };
    if let Some(val) = (IntStr { text, ..num }).parse_as_u64()
        && val <= max
    {
        let sp_data = sp.data();
        if !sp_data.ctxt.in_external_macro(cx.sess().source_map()) && sp_data.check_text(cx, |src| num.eq_str(src)) {
            let insert_pos = BytePos(sp_data.hi.0 - u32::from(max != 0xff) - 1);
            span_lint_and_then(
                cx,
                MISTYPED_LITERAL_SUFFIXES,
                Span::new(BytePos(insert_pos.0 - 1), sp_data.hi, sp_data.ctxt, sp_data.parent),
                "the final digit group looks like a type suffix",
                |diag| {
                    let insert_sp = Span::new(insert_pos, insert_pos, sp_data.ctxt, sp_data.parent);
                    if val <= (max >> 1) {
                        diag.span_suggestion_verbose(
                            insert_sp,
                            "change the final digit group to a signed type",
                            "i",
                            Applicability::MaybeIncorrect,
                        );
                    }
                    diag.span_suggestion_verbose(
                        insert_sp,
                        "change the final digit group to an unsigned type",
                        "u",
                        Applicability::MaybeIncorrect,
                    );
                    diag.span_suggestion(
                        num.trim_sp_to_digits(&sp_data),
                        "or adjust the digit groupings",
                        num.reformat_digits(),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
        true
    } else {
        false
    }
}

/// Returns `true` if the lint is emitted.
pub(super) fn check_float(cx: &EarlyContext<'_>, num: &FloatStr<'_>, sp: Span) -> bool {
    if let Some(exp) = num.exp_digits_str()
        && let [.., b'_', b'3', b'2'] | [.., b'_', b'6', b'4'] = exp.as_bytes()
    {
        let sp_data = sp.data();
        if !sp_data.ctxt.in_external_macro(cx.sess().source_map()) && sp_data.check_text(cx, |src| num.text == src) {
            span_lint_and_then(
                cx,
                MISTYPED_LITERAL_SUFFIXES,
                sp,
                "the final digit group looks like a type suffix",
                |diag| {
                    let insert_pos = BytePos(sp_data.hi.0 - 2);
                    diag.span_suggestion_verbose(
                        Span::new(insert_pos, insert_pos, sp_data.ctxt, sp_data.parent),
                        "change the final digit group to a float type",
                        "f",
                        Applicability::MaybeIncorrect,
                    );
                    diag.span_suggestion_verbose(
                        num.trim_sp_to_exp_digits(&sp_data),
                        "or remove the digit groups from the exponent",
                        exp.replace('_', ""),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
        true
    } else {
        false
    }
}

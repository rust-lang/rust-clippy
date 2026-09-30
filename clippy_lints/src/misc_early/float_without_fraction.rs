use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;
use rustc_span::Span;

use super::FLOAT_WITHOUT_FRACTION;

pub(super) fn check(cx: &EarlyContext<'_>, lit_span: Span, lit_snip: &str, suffix: &str) {
    let Some(before_suffix_index) = lit_snip.len().checked_sub(suffix.len() + 1) else {
        return;
    };

    if lit_snip.as_bytes()[before_suffix_index] == b'.' {
        span_lint_and_then(
            cx,
            FLOAT_WITHOUT_FRACTION,
            lit_span,
            "float literal should have a fraction after the dot",
            |diag| {
                diag.span_suggestion(
                    lit_span,
                    "add a zero",
                    format!("{}.0{suffix}", &lit_snip[..before_suffix_index]),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

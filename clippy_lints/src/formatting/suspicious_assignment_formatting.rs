use super::SUSPICIOUS_ASSIGNMENT_FORMATTING;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{FileRangeExt, SpanExt, StrExt};
use rustc_ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext};
use rustc_span::Span;

pub(super) fn check(cx: &EarlyContext<'_>, assign: &Expr, rhs: &Expr, op_sp: Span) {
    if let ExprKind::Unary(op, _) = rhs.kind
        && let assign_data = assign.span.data()
        && rhs.span.ctxt() == assign_data.ctxt
        && let op_data = op_sp.data()
        && op_data.ctxt == assign_data.ctxt
        && let op_str = op.as_str()
        && let sm = cx.sess().source_map()
        && !assign_data.ctxt.in_external_macro(sm)
        && let Some([lint_sp, sep_sp]) = op_data.map_range(sm, |scx, range| {
            let lint_range = range
                .extend_end_to(scx, assign_data.hi_ctxt())?
                .map_range_text(scx, |src| {
                    src.split_multipart_prefix(["=", op_str])
                        .and_then(|[s, rest]| rest.starts_with(char::is_whitespace).then_some(s))
                })?;
            lint_range
                .clone()
                .with_trailing_whitespace(scx)
                .map(|sep_range| [lint_range, sep_range])
        })
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_ASSIGNMENT_FORMATTING,
            lint_sp,
            "this looks similar to a compound assignment operator",
            |diag| {
                diag.span_suggestion(
                    lint_sp,
                    "reverse the characters",
                    format!("{op_str}="),
                    Applicability::MaybeIncorrect,
                )
                .span_suggestion(
                    sep_sp,
                    "separate the characters",
                    format!("= {op_str}"),
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_expr_identity_function;
use clippy_utils::source::SpanRangeExt;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::MAP_ALL_ANY_IDENTITY;

pub(super) fn check(
    cx: &LateContext<'_>,
    map_call_span: Span,
    map_arg: &Expr<'_>,
    any_call_span: Span,
    any_arg: &Expr<'_>,
    method: &str,
) {
    if is_expr_identity_function(cx, any_arg)
        && let map_any_call_span = map_call_span.with_hi(any_call_span.hi())
        && let Some(map_arg) = map_arg.span.get_source_text(cx)
    {
        span_lint_and_sugg(
            cx,
            MAP_ALL_ANY_IDENTITY,
            map_any_call_span,
            format!("usage of `.map(…).{method}(identity)`"),
            format!("use `.{method}(…)` directly"),
            format!("{method}({map_arg})"),
            Applicability::MachineApplicable,
        );
    }
}

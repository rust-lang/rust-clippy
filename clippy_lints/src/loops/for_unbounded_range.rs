use super::FOR_UNBOUNDED_RANGE;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::higher;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

pub fn check<'tcx>(cx: &LateContext<'tcx>, arg: &'tcx Expr<'tcx>, span: Span) {
    if let Some(range) = higher::Range::hir(cx, arg)
        && let Some(range_start) = range.start
        && let None = range.end
        && let ty = cx.typeck_results().expr_ty_adjusted(range_start)
        && (ty.is_integral() || ty.is_char())
    {
        let until_max = format!("={ty}::MAX");

        span_lint_hir_and_then(
            cx,
            FOR_UNBOUNDED_RANGE,
            arg.hir_id,
            span,
            "for loop on unbounded range (`0..`)",
            |diag| {
                diag.span_suggestion_verbose(
                    arg.span.shrink_to_hi(),
                    "for loops over unbounded ranges will wrap around, consider using `start..=MAX` instead",
                    until_max,
                    rustc_errors::Applicability::MachineApplicable,
                );
            },
        );
    }
}

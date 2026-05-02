use super::UNNECESSARY_AS_SLICE;
use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::sym;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) {
    if cx
        .typeck_results()
        .expr_ty(recv)
        .peel_refs()
        .ty_adt_def()
        .is_some_and(|adt| cx.tcx.is_diagnostic_item(sym::Vec, adt.did()))
    {
        if let Some(as_slice_span) = expr.span.trim_start(recv.span) {
            span_lint_and_sugg(
                cx,
                UNNECESSARY_AS_SLICE,
                as_slice_span,
                "this `as_slice` is unnecessary and can be removed as the method immediately following exists on `Vec` too",
                "remove `as_slice`",
                String::new(),
                Applicability::MachineApplicable,
            );
        }
    }
}

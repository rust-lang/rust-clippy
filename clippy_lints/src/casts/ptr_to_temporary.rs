use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{BorrowKind, Expr, ExprKind, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::Adjust;

use super::PTR_TO_TEMPORARY;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    cast_expr: &'tcx Expr<'_>,
    cast_to: &'tcx Ty<'_>,
) {
    if matches!(cast_to.kind, TyKind::Ptr(_))
        && let ExprKind::AddrOf(BorrowKind::Ref, _, e) = cast_expr.kind
    {
        // rustc's criteria of a "temporary value", so this should be 100% accurate
        if !e.is_place_expr(|base| {
            cx.typeck_results()
                .adjustments()
                .get(base.hir_id)
                .is_some_and(|x| x.iter().any(|adj| matches!(adj.kind, Adjust::Deref(_))))
        }) {
            span_lint_and_help(
                cx,
                PTR_TO_TEMPORARY,
                expr.span,
                "raw pointer to a temporary value",
                None,
                "usage of this pointer will cause Undefined Behavior; create a local binding to make it longer lived",
            );
        }
    }
}

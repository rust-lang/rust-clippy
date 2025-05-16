use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::macros::HirNode;
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_hir::PatKind;
use rustc_lint::LateContext;

use super::MATCHES_INSTEAD_OF_EQ;

// TODO: Adjust the parameters as necessary
pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &rustc_hir::Expr<'_>,
    ex: &rustc_hir::Expr<'_>,
    arms: &[rustc_hir::Arm<'_>],
) {
    let Some(partialeq) = cx.tcx.get_diagnostic_item(sym::PartialEq) else {
        return;
    };
    let expr_type = cx.typeck_results().expr_ty(ex);
    if !implements_trait(cx, expr_type, partialeq, &[expr_type.into()]) {
        return;
    }
    if arms.into_iter().all(|arm| {
        !matches!(
            arm.pat.kind,
            PatKind::Or(..) | PatKind::Struct(..) | PatKind::TupleStruct(..)
        )
    }) {
        span_lint_and_help(
            cx,
            MATCHES_INSTEAD_OF_EQ,
            expr.span(),
            "This expression can be replaced with a == comparison",
            None,
            "This type implements PartialEq, so you can use == instead of matches!()",
        );
    }
}

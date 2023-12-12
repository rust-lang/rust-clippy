use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LintContext};
use rustc_middle::ty::Ty;
use clippy_utils::in_constant;

use super::CAST_INTEGER;

pub(super) fn check(
    cx: &LateContext<'_>, 
    expr: &Expr<'_>, 
    from_ty: Ty<'_>,
    to_ty: Ty<'_>
) {

    if !should_lint(cx, expr, from_ty, to_ty) {
        return;
    }

    span_lint_and_help(
        cx,
        CAST_INTEGER,
        expr.span,
        "Integer casts can introduce subtle surprises and should be done with From/TryFrom.",
        None,
        "Try T::from(_) or T::try_from(_) instead",
    )
}

fn should_lint(cx: &LateContext<'_>, expr: &Expr<'_>, cast_from: Ty<'_>, cast_to: Ty<'_>) -> bool {
    // Do not suggest using From in consts/statics until it is valid to do so (see #2267).
    if in_constant(cx, expr.hir_id) {
        return false;
    }

    match (cast_from.is_integral(), cast_to.is_integral()) {
        (true, true) => true,
        (_, _) => false,
    }
}
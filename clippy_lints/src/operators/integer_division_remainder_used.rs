use clippy_utils::diagnostics::span_lint;
use rustc_ast::BinOpKind;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self};

use super::INTEGER_DIVISION_REMAINDER_USED;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, op: BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    if let BinOpKind::Div | BinOpKind::Rem = op
        && let lhs_ty = cx.typeck_results().expr_ty(lhs)
        && let rhs_ty = cx.typeck_results().expr_ty(rhs)
        && let ty::Int(_) | ty::Uint(_) = lhs_ty.peel_refs().kind()
        && let ty::Int(_) | ty::Uint(_) = rhs_ty.peel_refs().kind()
    {
        span_lint(
            cx,
            INTEGER_DIVISION_REMAINDER_USED,
            expr.span.source_callsite(),
            format!("use of `{}` has been disallowed in this context", op.as_str()),
        );
    }
}

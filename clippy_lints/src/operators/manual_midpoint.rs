use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::{is_floating_point_integer_literal, is_integer_literal};
use rustc_ast::BinOpKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

use super::MANUAL_MIDPOINT;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
    msrv: &Msrv,
) {
    if msrv.meets(msrvs::UINT_FLOAT_MIDPOINT)
        && op == BinOpKind::Div
        && (is_integer_literal(right, 2) || is_floating_point_integer_literal(right, 2))
        && let ExprKind::Binary(left_op, ll_expr, lr_expr) = left.kind
        && left_op.node == BinOpKind::Add
        && let left_ty = cx.typeck_results().expr_ty_adjusted(ll_expr)
        && let right_ty = cx.typeck_results().expr_ty_adjusted(lr_expr)
        && left_ty == right_ty
        && matches!(left_ty.kind(), ty::Uint(_) | ty::Float(_))
    {
        let mut app = Applicability::MachineApplicable;
        let left_sugg = Sugg::hir_with_applicability(cx, ll_expr, "..", &mut app);
        let right_sugg = Sugg::hir_with_applicability(cx, lr_expr, "..", &mut app);
        let sugg = format!("{left_ty}::midpoint({left_sugg}, {right_sugg})");
        span_lint_and_sugg(
            cx,
            MANUAL_MIDPOINT,
            expr.span,
            "manual implementation of `midpoint`",
            "use instead",
            sugg,
            app,
        );
    }
}

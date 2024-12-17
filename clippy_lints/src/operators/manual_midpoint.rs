use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::{is_floating_point_integer_literal, is_integer_literal};
use rustc_ast::{BinOpKind, LitKind};
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
        && !left.span.from_expansion()
        && !right.span.from_expansion()
        && op == BinOpKind::Div
        && (is_integer_literal(right, 2) || is_floating_point_integer_literal(right, 2))
        && let Some((ll_expr, lr_expr)) = add_operands(left)
        && add_operands(ll_expr).is_none() && add_operands(lr_expr).is_none()
        && let left_ty = cx.typeck_results().expr_ty_adjusted(ll_expr)
        && let right_ty = cx.typeck_results().expr_ty_adjusted(lr_expr)
        && left_ty == right_ty
        // Do not lint on `(_+1)/2` and `(1+_)/2`, it is likely a `div_ceil()` operation
        && !is_1_integer_literal(ll_expr) && !is_1_integer_literal(lr_expr)
        // FIXME: Also lint on signed integers when rust-lang/rust#134340 is merged
        && matches!(left_ty.kind(), ty::Uint(_) | ty::Float(_))
    {
        let mut app = Applicability::MachineApplicable;
        let left_sugg = Sugg::hir_with_context(cx, ll_expr, expr.span.ctxt(), "..", &mut app);
        let right_sugg = Sugg::hir_with_context(cx, lr_expr, expr.span.ctxt(), "..", &mut app);
        let sugg = format!("{left_ty}::midpoint({left_sugg}, {right_sugg})");
        span_lint_and_sugg(
            cx,
            MANUAL_MIDPOINT,
            expr.span,
            "manual implementation of `midpoint` which can overflow",
            format!("use `{left_ty}::midpoint` instead"),
            sugg,
            app,
        );
    }
}

/// Return the left and right operands if `expr` represents an addition
fn add_operands<'e, 'tcx>(expr: &'e Expr<'tcx>) -> Option<(&'e Expr<'tcx>, &'e Expr<'tcx>)> {
    match expr.kind {
        ExprKind::Binary(op, left, right) if op.node == BinOpKind::Add => Some((left, right)),
        _ => None,
    }
}

/// Check if the expression represents the integer 1 literal
fn is_1_integer_literal(expr: &Expr<'_>) -> bool {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Int(value, _) = lit.node
    {
        value.get() == 1
    } else {
        false
    }
}

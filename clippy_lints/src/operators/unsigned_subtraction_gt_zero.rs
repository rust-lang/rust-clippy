use clippy_utils::consts::is_zero_integer_const;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::{Sugg, make_binop};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

use super::UNSIGNED_SUBTRACTION_GT_ZERO;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    op: BinOpKind,
    lhs_expr: &'tcx Expr<'tcx>,
    rhs_expr: &'tcx Expr<'tcx>,
) {
    // Avoid linting macro-generated code to reduce noise
    if expr.span.from_expansion() {
        return;
    }

    // Only consider strict relational comparisons where one side is zero and the other is a subtraction
    let sub_expr = match op {
        // x > 0
        BinOpKind::Gt if is_zero_integer_const(cx, rhs_expr, expr.span.ctxt()) => lhs_expr,
        // 0 < x
        BinOpKind::Lt if is_zero_integer_const(cx, lhs_expr, expr.span.ctxt()) => rhs_expr,

        _ => return,
    };

    // Ensure the compared expression is a subtraction
    let (lhs, rhs) = match sub_expr.kind {
        ExprKind::Binary(sub_op, lhs, rhs) if sub_op.node == BinOpKind::Sub => (lhs, rhs),
        _ => return,
    };

    // Subtraction result type must be an unsigned primitive
    if !matches!(cx.typeck_results().expr_ty(sub_expr).peel_refs().kind(), ty::Uint(_)) {
        return;
    }

    // Build suggestions
    let mut app = Applicability::MaybeIncorrect;
    let left_sugg = Sugg::hir_with_applicability(cx, lhs, "_", &mut app);
    let right_sugg = Sugg::hir_with_applicability(cx, rhs, "_", &mut app);

    let replacement = make_binop(BinOpKind::Gt, &left_sugg, &right_sugg);
    let neq_suggestion = make_binop(BinOpKind::Ne, &left_sugg, &right_sugg);

    span_lint_and_then(
        cx,
        UNSIGNED_SUBTRACTION_GT_ZERO,
        expr.span,
        "suspicious comparison of unsigned subtraction to zero",
        |diag| {
            diag.help(format!("`{left_sugg} - {right_sugg} > 0` will panic in debug mode when `{left_sugg} < {right_sugg}` and wrap in release mode; `{replacement}` is clearer and will never panic"));
            diag.help(format!("if you meant inequality, use `{neq_suggestion}`"));
            diag.span_suggestion(expr.span, "try", replacement, app);
        },
    );
}

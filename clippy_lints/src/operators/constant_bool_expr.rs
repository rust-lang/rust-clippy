use clippy_utils::diagnostics::span_lint;
use clippy_utils::eq_expr_value;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;

use super::CONSTANT_BOOL_EXPR;

pub(crate) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    match op {
        BinOpKind::Or => detect_always_true_or(cx, e, left, right),
        BinOpKind::And => detect_always_false_and(cx, e, left, right),
        _ => (),
    }
}

// `left_lhs != left_rhs || right_lhs != right_rhs`
fn detect_always_true_or<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, left: &'tcx Expr<'_>, right: &'tcx Expr<'_>) {
    let ExprKind::Binary(left_op, left_lhs, left_rhs) = left.kind else {
        return;
    };
    let ExprKind::Binary(right_op, right_lhs, right_rhs) = right.kind else {
        return;
    };
    if left_op.node != BinOpKind::Ne || right_op.node != BinOpKind::Ne {
        return;
    }

    if detect_variants(cx, e, left_lhs, right_lhs, left_rhs, right_rhs) {
        span_lint(cx, CONSTANT_BOOL_EXPR, e.span, "expression always evaluates to `true`");
    }
}

// `left_lhs == left_rhs && right_lhs == right_rhs`
fn detect_always_false_and<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    let ExprKind::Binary(left_op, left_lhs, left_rhs) = left.kind else {
        return;
    };
    let ExprKind::Binary(right_op, right_lhs, right_rhs) = right.kind else {
        return;
    };
    if left_op.node != BinOpKind::Eq || right_op.node != BinOpKind::Eq {
        return;
    }

    if detect_variants(cx, e, left_lhs, right_lhs, left_rhs, right_rhs) {
        span_lint(cx, CONSTANT_BOOL_EXPR, e.span, "expression always evaluates to `false`");
    }
}

fn detect_variants<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    left_lhs: &'tcx Expr<'_>,
    right_lhs: &'tcx Expr<'_>,
    left_rhs: &'tcx Expr<'_>,
    right_rhs: &'tcx Expr<'_>,
) -> bool {
    let ctxt = e.span.ctxt();

    let detect_variant = |left_a, right_a, left_literal: &Expr<'_>, right_literal: &Expr<'_>| {
        matches!(left_literal.kind, ExprKind::Lit(_))
            && matches!(right_literal.kind, ExprKind::Lit(_))
            && !eq_expr_value(cx, ctxt, left_literal, right_literal)
            && eq_expr_value(cx, ctxt, left_a, right_a)
    };

    // (a CMP_OP _) BIN_OP (a CMP_OP _)
    detect_variant(left_lhs, right_lhs, left_rhs, right_rhs)
        // (a CMP_OP _) BIN_OP (_ CMP_OP a)
        || detect_variant(left_lhs, right_rhs, left_rhs, right_lhs)
        // (_ CMP_OP a) BIN_OP (a CMP_OP _)
        || detect_variant(left_rhs, right_lhs, left_lhs, right_rhs)
        // (_ CMP_OP a) BIN_OP (_ CMP_OP a)
        || detect_variant(left_rhs, right_rhs, left_lhs, right_lhs)
}

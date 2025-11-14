use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::SpanRangeExt;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_hir::{AssignOpKind, BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;
use rustc_span::source_map::Spanned;

use super::DECIMAL_BITWISE_OPERANDS;

fn check_bitwise_binary_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::Binary(op, left, right) = &expr.kind
        && matches!(op.node, BinOpKind::BitAnd | BinOpKind::BitOr | BinOpKind::BitXor)
    {
        for expr in [left, right] {
            if let ExprKind::Lit(lit) = &expr.kind
                && is_decimal_number(cx, lit.span)
                && !is_power_of_twoish(lit)
            {
                emit_lint(cx, lit.span);
            }
        }
    }
}

fn check_bitwise_assign_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::AssignOp(op, _, e) = &expr.kind
        && matches!(
            op.node,
            AssignOpKind::BitAndAssign | AssignOpKind::BitOrAssign | AssignOpKind::BitXorAssign
        )
        && let ExprKind::Lit(lit) = e.kind
        && is_decimal_number(cx, lit.span)
        && !is_power_of_twoish(&lit)
    {
        emit_lint(cx, lit.span);
    }
}

fn is_decimal_number(cx: &LateContext<'_>, span: Span) -> bool {
    span.check_source_text(cx, |src| {
        !(src.starts_with("0b") || src.starts_with("0x") || src.starts_with("0o"))
    })
}

fn is_power_of_twoish(lit: &Spanned<LitKind>) -> bool {
    if let LitKind::Int(Pu128(val), _) = lit.node {
        return val.is_power_of_two() || val.wrapping_add(1).is_power_of_two();
    }
    false
}

fn emit_lint(cx: &LateContext<'_>, span: Span) {
    span_lint_and_help(
        cx,
        DECIMAL_BITWISE_OPERANDS,
        span,
        "using decimal literal for bitwise operation",
        None,
        "use binary (0b...), hex (0x...), or octal (0o...) notation for better readability",
    );
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
    if expr.span.from_expansion() {
        return;
    }
    check_bitwise_binary_expr(cx, expr);
    check_bitwise_assign_expr(cx, expr);
}

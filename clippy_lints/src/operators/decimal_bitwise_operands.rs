use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::SpanRangeExt;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_hir::{AssignOpKind, BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;

use super::DECIMAL_BITWISE_OPERANDS;

pub(super) fn check_binary<'tcx>(cx: &LateContext<'tcx>, op: BinOpKind, left: &'tcx Expr<'_>, right: &'tcx Expr<'_>) {
    if !matches!(op, BinOpKind::BitAnd | BinOpKind::BitOr | BinOpKind::BitXor) {
        return;
    }

    for expr in [left, right] {
        if let ExprKind::Lit(lit) = &expr.kind
            && is_decimal_number(cx, lit.span)
            && !is_power_of_twoish(lit.node)
        {
            emit_lint(cx, lit.span);
        }
    }
}

pub(super) fn check_assign<'tcx>(cx: &LateContext<'tcx>, op: AssignOpKind, rhs: &'tcx Expr<'_>) {
    if matches!(
        op,
        AssignOpKind::BitAndAssign | AssignOpKind::BitOrAssign | AssignOpKind::BitXorAssign
    ) && let ExprKind::Lit(lit) = &rhs.kind
        && is_decimal_number(cx, lit.span)
        && !is_power_of_twoish(lit.node)
    {
        emit_lint(cx, lit.span);
    }
}

fn is_decimal_number(cx: &LateContext<'_>, span: Span) -> bool {
    span.check_source_text(cx, |src| {
        !(src.starts_with("0b") || src.starts_with("0x") || src.starts_with("0o"))
    })
}

fn is_power_of_twoish(node: LitKind) -> bool {
    if let LitKind::Int(Pu128(val), _) = node {
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

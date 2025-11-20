use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::SpanRangeExt;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;

use super::DECIMAL_BITWISE_OPERANDS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, op: BinOpKind, left: &'tcx Expr<'_>, right: &'tcx Expr<'_>) {
    if !matches!(op, BinOpKind::BitAnd | BinOpKind::BitOr | BinOpKind::BitXor) {
        return;
    }

    for expr in [left, right] {
        if let ExprKind::Lit(lit) = &expr.kind
            && let LitKind::Int(Pu128(val), _) = lit.node
            && is_decimal_number(cx, lit.span)
            && !is_single_digit(val)
            && !is_power_of_twoish(val)
        {
            emit_lint(cx, lit.span, val);
        }
    }
}

fn is_decimal_number(cx: &LateContext<'_>, span: Span) -> bool {
    span.check_source_text(cx, |src| {
        !(src.starts_with("0b") || src.starts_with("0x") || src.starts_with("0o"))
    })
}

fn is_power_of_twoish(val: u128) -> bool {
    val.is_power_of_two() || val.wrapping_add(1).is_power_of_two()
}

fn is_single_digit(val: u128) -> bool {
    val <= 9
}

fn emit_lint(cx: &LateContext<'_>, span: Span, val: u128) {
    span_lint_and_help(
        cx,
        DECIMAL_BITWISE_OPERANDS,
        span,
        "using decimal literal for bitwise operation",
        None,
        format!("use binary (0b{val:b}), hex (0x{val:x}), or octal (0o{val:o}) notation for better readability"),
    );
}

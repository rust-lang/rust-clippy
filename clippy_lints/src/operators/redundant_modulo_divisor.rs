use clippy_utils::SpanlessEq;
use clippy_utils::diagnostics::span_lint;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;

use super::REDUNDANT_MODULO_DIVISOR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, op: BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    if op != BinOpKind::Rem
        || !cx.typeck_results().expr_ty(lhs).peel_refs().is_integral()
        || !cx.typeck_results().expr_ty(rhs).peel_refs().is_integral()
    {
        return;
    }

    let mut term_count = 0;
    let mut found_divisor = false;
    if !check_add_chain(
        cx,
        expr.span.ctxt(),
        lhs,
        rhs,
        &mut term_count,
        &mut found_divisor,
        true,
    )
        || term_count < 3
        || !found_divisor
    {
        return;
    }

    span_lint(
        cx,
        REDUNDANT_MODULO_DIVISOR,
        expr.span,
        "left-hand side of modulo contains an addition of the divisor",
    );
}

fn check_add_chain(
    cx: &LateContext<'_>,
    ctxt: rustc_span::SyntaxContext,
    expr: &Expr<'_>,
    divisor: &Expr<'_>,
    term_count: &mut usize,
    found_divisor: &mut bool,
    split_add: bool,
) -> bool {
    if !cx.typeck_results().expr_ty(expr).peel_refs().is_integral() {
        return false;
    }
    *found_divisor |= SpanlessEq::new(cx).eq_expr(ctxt, expr, divisor);

    if let ExprKind::Binary(op, lhs, rhs) = expr.kind
        && op.node == BinOpKind::Add
        // Split left-associated chains like `x + n + y`, but keep grouped RHS
        // additions like `x + (n + 1)` as a single additive term.
        && split_add
    {
        check_add_chain(cx, ctxt, lhs, divisor, term_count, found_divisor, true)
            && check_add_chain(cx, ctxt, rhs, divisor, term_count, found_divisor, false)
    } else {
        *term_count += 1;
        true
    }
}

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Expr, ExprKind, UnOp};
use rustc_lint::EarlyContext;

use super::DOUBLE_NEG;

pub(super) fn check(cx: &EarlyContext<'_>, expr: &Expr) {
    if let ExprKind::Unary(UnOp::Neg, ref inner) = expr.kind {
        if let ExprKind::Unary(UnOp::Neg, _) = inner.kind {
            span_lint_and_help(
                cx,
                DOUBLE_NEG,
                expr.span,
                "`--x`, which is a double negation of `x` and not a pre-decrement operator \
                as in C/C++. Rust does not have unary increment (++) or decrement (--) operators \
                at the moment",
                None,
                "consider using `x -= 1` if you intended to decrease the value of `x`",
            );
        }
    }
}

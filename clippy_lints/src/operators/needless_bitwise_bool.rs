use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_hir::{BinOp, BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;

use super::NEEDLESS_BITWISE_BOOL;

pub(super) fn check(cx: &LateContext<'_>, e: &Expr<'_>, op: BinOp, rhs: &Expr<'_>) {
    let op_str = match op.node {
        BinOpKind::BitAnd => "&&",
        BinOpKind::BitOr => "||",
        _ => return,
    };
    if matches!(
        rhs.kind,
        ExprKind::Call(..) | ExprKind::MethodCall(..) | ExprKind::Binary(..) | ExprKind::Unary(..)
    ) && cx.typeck_results().expr_ty(e).is_bool()
        && !rhs.can_have_side_effects()
    {
        span_lint_and_then(
            cx,
            NEEDLESS_BITWISE_BOOL,
            e.span,
            "use of bitwise operator instead of lazy operator between booleans",
            |diag| {
                diag.span_suggestion_verbose(op.span, "try", op_str, Applicability::MachineApplicable);
            },
        );
    }
}

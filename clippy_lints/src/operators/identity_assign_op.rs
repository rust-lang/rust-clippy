use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr};
use rustc_lint::LateContext;

use super::IDENTITY_ASSIGN_OP;

// TODO: Adjust the parameters as necessary
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    if match op {
        BinOpKind::Add | BinOpKind::Sub | BinOpKind::BitOr | BinOpKind::BitXor | BinOpKind::Shl | BinOpKind::Shr => {
            matches!(ConstEvalCtxt::new(cx).eval(right), Some(Constant::Int(0)))
        },

        BinOpKind::Mul | BinOpKind::Div => {
            matches!(ConstEvalCtxt::new(cx).eval(right), Some(Constant::Int(1)))
        },

        _ => false,
    } {
        span_lint_and_sugg(
            cx,
            IDENTITY_ASSIGN_OP,
            expr.span,
            "this assignment operation has no effect",
            "consider replacing it with",
            snippet(cx, left.span, "..").to_string(),
            Applicability::MachineApplicable,
        );
    }
}

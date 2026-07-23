use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, Stmt};
use rustc_lint::LateContext;

use super::IDENTITY_ASSIGN_OP;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'_>,
    _expr: &'tcx Expr<'_>,
    op: BinOpKind,
    _left: &'tcx Expr<'_>,
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
            stmt.span,
            "this operation has no effect",
            "remove it",
            String::new(),
            Applicability::MachineApplicable,
        );
    }
}

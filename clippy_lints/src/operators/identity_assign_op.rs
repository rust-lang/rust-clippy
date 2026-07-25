use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eq_expr_value;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, Node, Stmt, StmtKind};
use rustc_lint::LateContext;

use super::IDENTITY_ASSIGN_OP;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    let is_identity = match op {
        BinOpKind::Add | BinOpKind::Sub | BinOpKind::BitOr | BinOpKind::BitXor | BinOpKind::Shl | BinOpKind::Shr => {
            is_zero_or_one(cx, right, 0)
        },

        BinOpKind::Mul | BinOpKind::Div => is_zero_or_one(cx, right, 1),

        _ => false,
    };

    if is_identity && !is_part_of_series(cx, stmt, op, left) {
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

fn is_part_of_series(cx: &LateContext<'_>, stmt: &Stmt<'_>, op: BinOpKind, left: &Expr<'_>) -> bool {
    let Node::Block(block) = cx.tcx.parent_hir_node(stmt.hir_id) else {
        return false;
    };
    let Some(index) = block.stmts.iter().position(|other| other.hir_id == stmt.hir_id) else {
        return false;
    };

    (index > 0 && is_matching_assign_op(cx, &block.stmts[index - 1], op, left))
    ||
    (index < block.stmts.len() - 1 && is_matching_assign_op(cx, &block.stmts[index + 1], op, left))
}

fn is_matching_assign_op(cx: &LateContext<'_>, stmt: &Stmt<'_>, op: BinOpKind, left: &Expr<'_>) -> bool {
    let StmtKind::Semi(expr) = stmt.kind else {
        return false;
    };
    let ExprKind::AssignOp(other_op, other_left, _) = expr.kind else {
        return false;
    };
    let other_op: BinOpKind = other_op.node.into();

    other_op == op && eq_expr_value(cx, left.span.ctxt(), left, other_left)
}

fn is_zero_or_one(cx: &LateContext<'_>, expr: &Expr<'_>, expected: u128) -> bool {
    const F16_ZERO: u16 = 0.0_f16.to_bits();
    const F16_ONE: u16 = 1.0_f16.to_bits();
    const F128_ZERO: u128 = 0.0_f128.to_bits();
    const F128_ONE: u128 = 1.0_f128.to_bits();

    let Some(value) = ConstEvalCtxt::new(cx).eval(expr).map(Constant::peel_refs) else {
        return false;
    };

    match value {
        Constant::Int(value) => value == expected,
        Constant::F16(value) => value == if expected == 0 { F16_ZERO } else { F16_ONE },
        Constant::F32(value) => value == if expected == 0 { 0.0 } else { 1.0 },
        Constant::F64(value) => value == if expected == 0 { 0.0 } else { 1.0 },
        Constant::F128(value) => value == if expected == 0 { F128_ZERO } else { F128_ONE },
        _ => false,
    }
}

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::LitKind;
use rustc_hir::ExprKind::Lit;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::LateContext;

use super::TRIVIAL_VAR_PRIMITIVE_DISJUNCTION;

fn context_applicable<'a>(expr: &'a Expr<'a>) -> Option<(Res, LitKind)> {
    if let ExprKind::Binary(new_op, new_l, new_r) = expr.kind {
        if new_op.node == BinOpKind::Ne {
            normalize_expression(new_l, new_r)
        } else {
            None
        }
    } else {
        None
    }
}

fn normalize_expression<'a>(l: &'a Expr<'a>, r: &'a Expr<'a>) -> Option<(Res, LitKind)> {
    if let (ExprKind::Path(QPath::Resolved(_, path)), Lit(lit)) = (&l.kind, &r.kind) {
        Some((path.res, lit.node))
    } else if let (Lit(lit), ExprKind::Path(QPath::Resolved(_, path))) = (&l.kind, &r.kind) {
        Some((path.res, lit.node))
    } else {
        None
    }
}

pub(super) fn check(cx: &LateContext<'_>, e: &Expr<'_>, left: &Expr<'_>, right: &Expr<'_>, op: BinOpKind) {
    if let BinOpKind::Or = op {
        let lhs = context_applicable(left);
        let rhs = context_applicable(right);

        if let (Some((lhs_var, lhs_lit)), Some((rhs_var, rhs_lit))) = (lhs, rhs)
            && lhs_var == rhs_var
            && lhs_lit != rhs_lit
        {
            span_lint_and_help(
                cx,
                TRIVIAL_VAR_PRIMITIVE_DISJUNCTION,
                e.span,
                "this expression will always evaluate as true",
                None,
                "the wrong variables or operators might have been used",
            );
        }
    }
}

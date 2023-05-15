use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::ExprKind;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, TypeAndMut};

use super::AS_PTR_CAST_UNDERSCORE;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Cast(cast_expr, ..) = expr.kind;
        let (cast_from, cast_to) = (cx.typeck_results().expr_ty(cast_expr), cx.typeck_results().expr_ty(expr));
        if let ty::RawPtr(TypeAndMut { mutbl: from_mutbl, .. }) = cast_from.kind();
        // check both mutability and type are the same
        if cast_from.kind() == cast_to.kind();
        then {
            let constness = match *from_mutbl {
                Mutability::Not => "const",
                Mutability::Mut => "mut",
            };

            span_lint_and_help(
                cx,
                AS_PTR_CAST_UNDERSCORE,
                expr.span,
                &format!("casting a raw pointer using `as *{constness} _` without changing type or constness"),
                None,
                "this is an extrenuous operation",
            );
        }
    }
}

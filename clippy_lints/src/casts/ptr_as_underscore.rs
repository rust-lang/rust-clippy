use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::ExprKind;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, TypeAndMut};

use super::PTR_AS_UNDERSCORE;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Cast(cast_expr, cast_to_hir_ty) = expr.kind;
        let (cast_from, cast_to) = (cx.typeck_results().expr_ty(cast_expr), cx.typeck_results().expr_ty(expr));
        // if let ty::RawPtr(TypeAndMut { mutbl: from_mutbl, .. }) = cast_from.kind();
        // if let ty::Infer(_) = cast_to.kind();
        then {
            span_lint_and_help(
                cx,
                PTR_AS_UNDERSCORE,
                expr.span,
                &format!("using `as * _` conversion\n{:#?}\n{:#?}", cast_from.kind(), cast_to.kind()),
                None,
                "this is likely an extraneous operation",
            );
        }
    }

    if_chain! {
        if let ExprKind::Cast(cast_expr, _) = expr.kind;
        let (cast_from, cast_to) = (cx.typeck_results().expr_ty(cast_expr), cx.typeck_results().expr_ty(expr));
        if let ty::RawPtr(TypeAndMut { mutbl: from_mutbl, .. }) = cast_from.kind();
        if let ty::Infer(_) = cast_to.kind();
        then {
            let constness = match *from_mutbl {
                Mutability::Not => "const",
                Mutability::Mut => "mut",
            };

            span_lint_and_help(
                cx,
                PTR_AS_UNDERSCORE,
                expr.span,
                &format!("using `as *{constness} _` conversion"),
                None,
                "this is likely an extraneous operation",
            );
        }
    }
}

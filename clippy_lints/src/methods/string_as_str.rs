use clippy_utils::get_parent_expr;
use clippy_utils::sugg::Sugg;
use clippy_utils::{diagnostics::span_lint_and_sugg, ty::is_type_diagnostic_item};
use rustc_ast::BorrowKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym::String;

use super::STRING_AS_STR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) {
    if is_string(cx, recv) && !is_within_borrowed_call(cx, expr) {
        let parent = get_parent_expr(cx, recv).unwrap();
        let sugg = format!("&{}", Sugg::hir(cx, recv, "..").maybe_par());
        span_lint_and_sugg(
            cx,
            STRING_AS_STR,
            parent.span,
            "used `as_str()` for reference",
            "replace it with",
            sugg,
            Applicability::MaybeIncorrect,
        );
    }
}

fn is_string(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(expr), String)
}

fn is_within_borrowed_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(parent) = clippy_utils::get_parent_expr(cx, expr) {
        return matches!(
            parent.kind,
            ExprKind::Closure(..)
                | ExprKind::Tup(..)
                | ExprKind::Match(_, _, _)
                | ExprKind::AddrOf(BorrowKind::Ref, ..)
                | ExprKind::MethodCall(_, _, _, _)
        );
    }

    false
}

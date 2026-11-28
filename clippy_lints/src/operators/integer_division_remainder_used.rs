use super::INTEGER_DIVISION_REMAINDER_USED;
use clippy_utils::diagnostics::span_lint;
use clippy_utils::sym;
use rustc_ast::BinOpKind;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LintContext as _};
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

pub(super) fn check(cx: &LateContext<'_>, op: BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>, span: Span) {
    if let BinOpKind::Div | BinOpKind::Rem = op
        && let lhs_ty = cx.typeck_results().expr_ty(lhs)
        && let rhs_ty = cx.typeck_results().expr_ty(rhs)
        && let ty::Int(_) | ty::Uint(_) = lhs_ty.peel_refs().kind()
        && let ty::Int(_) | ty::Uint(_) = rhs_ty.peel_refs().kind()
        && !span.in_external_macro(cx.sess().source_map())
    {
        span_lint(
            cx,
            INTEGER_DIVISION_REMAINDER_USED,
            span,
            format!("use of `{}` has been disallowed in this context", op.as_str()),
        );
    }
}
// check method call is present in specific list if yes also lint it
pub(super) fn check_method_call(cx: &LateContext<'_>, method_name: Symbol, receiver: &Expr<'_>, span: Span) {
    if matches!(
        method_name,
        sym::checked_div
            | sym::checked_div_euclid
            | sym::checked_div_exact
            | sym::checked_rem
            | sym::checked_rem_euclid
            | sym::div_ceil
            | sym::div_euclid
            | sym::div_exact
            | sym::overflowing_div
            | sym::overflowing_div_euclid
            | sym::overflowing_rem
            | sym::overflowing_rem_euclid
            | sym::rem_euclid
            | sym::strict_div
            | sym::strict_div_euclid
            | sym::saturating_div
            | sym::strict_rem
            | sym::strict_rem_euclid
            | sym::wrapping_div
            | sym::wrapping_div_euclid
            | sym::wrapping_rem
            | sym::wrapping_rem_euclid,
    ) {
        let instance_ty = cx.typeck_results().expr_ty(receiver);
        if matches!(instance_ty.peel_refs().kind(), ty::Int(_) | ty::Uint(_)) {
            span_lint(
                cx,
                INTEGER_DIVISION_REMAINDER_USED,
                span.source_callsite(),
                format!("use of `{method_name}` has been disallowed in this context"),
            );
        }
    }
}

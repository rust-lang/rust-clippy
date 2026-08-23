use clippy_utils::diagnostics::span_lint;
use rustc_ast::BinOpKind;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;

use super::INTEGER_DIVISION_REMAINDER_USED;

pub(super) fn check(cx: &LateContext<'_>, op: BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>, span: Span) {
    if let BinOpKind::Div | BinOpKind::Rem = op
        && let lhs_ty = cx.typeck_results().expr_ty(lhs)
        && let rhs_ty = cx.typeck_results().expr_ty(rhs)
        && let ty::Int(_) | ty::Uint(_) = lhs_ty.peel_refs().kind()
        && let ty::Int(_) | ty::Uint(_) = rhs_ty.peel_refs().kind()
    {
        span_lint(
            cx,
            INTEGER_DIVISION_REMAINDER_USED,
            span.source_callsite(),
            format!("use of `{}` has been disallowed in this context", op.as_str()),
        );
    }
}
// check method call is present in specific list if yes also lint it
pub(super) fn check_method_call(cx: &LateContext<'_>, method_name: &str, receiver: &Expr<'_>, span: Span) {
    let instance_ty = cx.typeck_results().expr_ty(receiver);
    if matches!(
        method_name,
        "checked_div"
            | "checked_div_euclid"
            | "checked_div_exact"
            | "checked_rem"
            | "checked_rem_euclid"
            | "div_ceil"
            | "div_euclid"
            | "div_exact"
            | "overflowing_div"
            | "overflowing_div_euclid"
            | "overflowing_rem"
            | "overflowing_rem_euclid"
            | "rem_euclid"
            | "strict_div"
            | "strict_div_euclid"
            | "saturating_div"
            | "strict_rem"
            | "strict_rem_euclid"
            | "wrapping_div"
            | "wrapping_div_euclid"
            | "wrapping_rem"
            | "wrapping_rem_euclid"
    ) && matches!(instance_ty.peel_refs().kind(), ty::Int(_) | ty::Uint(_))
    {
        span_lint(
            cx,
            INTEGER_DIVISION_REMAINDER_USED,
            span.source_callsite(),
            format!("use of `{method_name}` has been disallowed in this context"),
        );
    }
}

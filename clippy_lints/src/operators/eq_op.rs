use clippy_utils::ast_utils::is_useless_with_eq_exprs;
use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::macros::{find_assert_eq_args, first_node_macro_backtrace};
use clippy_utils::{eq_expr_value, is_in_test_function, sym};
use rustc_ast::LitKind;
use rustc_hir::{BinOpKind, Expr, ExprKind, Lit};
use rustc_lint::LateContext;

use super::EQ_OP;

/// Checks if `l` and `r` are two literals using different forms (e.g., `b' '` vs. `0x20u8`)
fn are_different_form_literals(l: &Lit, r: &Lit) -> bool {
    match (l.node, r.node) {
        (LitKind::Str(_, l_style), LitKind::Str(_, r_style))
        | (LitKind::ByteStr(_, l_style), LitKind::ByteStr(_, r_style))
        | (LitKind::CStr(_, l_style), LitKind::CStr(_, r_style)) => l_style != r_style,
        (LitKind::Int(_, l_int_type), LitKind::Int(_, r_int_type)) => l_int_type != r_int_type,
        (LitKind::Float(_, l_float_type), LitKind::Float(_, r_float_type)) => l_float_type != r_float_type,
        (LitKind::Byte(_), LitKind::Byte(_))
        | (LitKind::Char(_), LitKind::Char(_))
        | (LitKind::Bool(_), LitKind::Bool(_)) => false,
        _ => true,
    }
}

pub(crate) fn check_assert<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
    if let Some(macro_call) = first_node_macro_backtrace(cx, e).find(|macro_call| {
        matches!(
            cx.tcx.get_diagnostic_name(macro_call.def_id),
            Some(sym::assert_eq_macro | sym::assert_ne_macro | sym::debug_assert_eq_macro | sym::debug_assert_ne_macro)
        )
    }) && let Some((lhs, rhs, _)) = find_assert_eq_args(cx, e, macro_call.expn)
        && eq_expr_value(cx, lhs, rhs)
        && macro_call.is_local()
        && !is_in_test_function(cx.tcx, e.hir_id)
        && !matches!((lhs.kind, rhs.kind), (ExprKind::Lit(l), ExprKind::Lit(r)) if are_different_form_literals(&l, &r))
    {
        span_lint(
            cx,
            EQ_OP,
            lhs.span.to(rhs.span),
            format!(
                "identical args used in this `{}!` macro call",
                cx.tcx.item_name(macro_call.def_id)
            ),
        );
    }
}

pub(crate) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    if is_useless_with_eq_exprs(op)
        && eq_expr_value(cx, left, right)
        && !is_in_test_function(cx.tcx, e.hir_id)
        && !matches!((left.kind, right.kind), (ExprKind::Lit(l), ExprKind::Lit(r)) if are_different_form_literals(&l, &r))
    {
        span_lint_and_then(
            cx,
            EQ_OP,
            e.span,
            format!("equal expressions as operands to `{}`", op.as_str()),
            |diag| {
                if let BinOpKind::Ne = op
                    && cx.typeck_results().expr_ty(left).is_floating_point()
                {
                    diag.note("if you intended to check if the operand is NaN, use `.is_nan()` instead");
                }
            },
        );
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{indent_of, reindent_multiline, snippet};
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{is_expr_final_block_expr, is_expr_used_or_unified, peel_blocks};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::RETURN_AND_THEN;

fn is_final_call(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    if !is_expr_used_or_unified(cx.tcx, expr) {
        return false;
    }
    is_expr_final_block_expr(cx.tcx, expr)
}

/// lint if `and_then` is the last call in the function
pub(super) fn check<'tcx>(
    cx: &LateContext<'_>,
    expr: &hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    arg: &'tcx hir::Expr<'_>,
) {
    if !is_final_call(cx, expr) {
        return;
    }

    let is_option = is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Option);
    let is_result = is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Result);

    if !is_option && !is_result {
        return;
    }

    let hir::ExprKind::Closure(&hir::Closure { body, fn_decl, .. }) = arg.kind else {
        return;
    };

    let closure_arg = fn_decl.inputs[0];
    let closure_body = cx.tcx.hir().body(body);
    let closure_expr = peel_blocks(closure_body.value);

    let msg = "use the question mark operator instead of an `and_then` call";
    let body_snip = snippet(cx, closure_expr.span, "..");
    let inner = if body_snip.starts_with('{') {
        body_snip[1..body_snip.len() - 1].trim()
    } else {
        body_snip.trim()
    };

    let sugg = format!(
        "let {} = {}?;\n{}",
        snippet(cx, closure_arg.span, "_"),
        snippet(cx, recv.span, ".."),
        reindent_multiline(inner.into(), false, indent_of(cx, expr.span))
    );

    span_lint_and_sugg(
        cx,
        RETURN_AND_THEN,
        expr.span,
        msg,
        "try",
        sugg,
        Applicability::MachineApplicable,
    );
}

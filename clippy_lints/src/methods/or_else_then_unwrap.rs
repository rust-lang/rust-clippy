use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{is_res_lang_ctor, path_res};
use rustc_errors::Applicability;
use rustc_hir::lang_items::LangItem;
use rustc_hir::{Body, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::OR_ELSE_THEN_UNWRAP;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    unwrap_expr: &Expr<'_>,
    recv: &'tcx Expr<'tcx>,
    or_else_arg: &'tcx Expr<'_>,
    or_span: Span,
) {
    let ty = cx.typeck_results().expr_ty(recv); // get type of x (we later check if it's Option or Result)
    let title;
    let or_else_arg_content: Span;

    if is_type_diagnostic_item(cx, ty, sym::Option) {
        title = "found `.or_else(|| Some(…)).unwrap()`";
        if let Some(content) = get_content_if_ctor_matches_in_closure(cx, or_else_arg, LangItem::OptionSome) {
            or_else_arg_content = content;
        } else {
            return;
        }
    } else if is_type_diagnostic_item(cx, ty, sym::Result) {
        title = "found `.or_else(|| Ok(…)).unwrap()`";
        if let Some(content) = get_content_if_ctor_matches_in_closure(cx, or_else_arg, LangItem::ResultOk) {
            or_else_arg_content = content;
        } else {
            return;
        }
    } else {
        // Someone has implemented a struct with .or(...).unwrap() chaining,
        // but it's not an Option or a Result, so bail
        return;
    }

    let mut applicability = Applicability::MachineApplicable;
    let suggestion = format!(
        "unwrap_or_else(|| {})",
        snippet_with_applicability(cx, or_else_arg_content, "..", &mut applicability)
    );

    span_lint_and_sugg(
        cx,
        OR_ELSE_THEN_UNWRAP,
        unwrap_expr.span.with_lo(or_span.lo()),
        title,
        "try",
        suggestion,
        applicability,
    );
}

fn get_content_if_ctor_matches_in_closure(cx: &LateContext<'_>, expr: &Expr<'_>, item: LangItem) -> Option<Span> {
    if let ExprKind::Closure(closure) = expr.kind
        && let Body {
            params: [],
            value: body,
        } = cx.tcx.hir_body(closure.body)
        && let ExprKind::Call(some_expr, [arg]) = body.kind
        && is_res_lang_ctor(cx, path_res(cx, some_expr), item)
    {
        Some(arg.span.source_callsite())
    } else {
        None
    }
}

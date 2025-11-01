use crate::clippy_utils::res::{MaybeDef, MaybeQPath};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::lang_items::LangItem;
use rustc_hir::{Body, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::AdtDef;
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
    let (title, or_else_arg_content) = match ty
        .ty_adt_def()
        .map(AdtDef::did)
        .and_then(|did| cx.tcx.get_diagnostic_name(did))
    {
        Some(sym::Option)
            if let Some(content) = get_content_if_ctor_matches_in_closure(cx, or_else_arg, LangItem::OptionSome) =>
        {
            ("found `.or_else(|| Some(…)).unwrap()`", content)
        },
        Some(sym::Result)
            if let Some(content) = get_content_if_ctor_matches_in_closure(cx, or_else_arg, LangItem::ResultOk) =>
        {
            ("found `.or_else(|| Ok(…)).unwrap()`", content)
        },
        // Someone has implemented a struct with .or(...).unwrap() chaining,
        // but it's not an Option or a Result, so bail
        _ => return,
    };

    let mut applicability = Applicability::MachineApplicable;
    let suggestion = format!(
        "unwrap_or_else(|| {})",
        snippet_with_applicability(cx, or_else_arg_content, "..", &mut applicability)
    );

    let span = unwrap_expr.span.with_lo(or_span.lo());
    span_lint_and_then(cx, OR_ELSE_THEN_UNWRAP, span, title, |diag| {
        diag.span_suggestion_verbose(span, "try", suggestion, applicability);
    });
}

fn get_content_if_ctor_matches_in_closure(cx: &LateContext<'_>, expr: &Expr<'_>, item: LangItem) -> Option<Span> {
    if let ExprKind::Closure(closure) = expr.kind
        && let Body {
            params: [],
            value: body,
        } = cx.tcx.hir_body(closure.body)
        && let ExprKind::Call(some_or_ok, [arg]) = body.kind
        && some_or_ok.res(cx).ctor_parent(cx).is_lang_item(cx, item)
    {
        Some(arg.span.source_callsite())
    } else {
        None
    }
}

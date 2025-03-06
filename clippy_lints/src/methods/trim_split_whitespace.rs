use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::TRIM_SPLIT_WHITESPACE;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    recv: &'tcx Expr<'_>,
    call_span: rustc_span::Span,
) {
    let tyckres = cx.typeck_results();
    if let Some(split_ws_def_id) = tyckres.type_dependent_def_id(expr.hir_id)
        && cx.tcx.is_diagnostic_item(sym::str_split_whitespace, split_ws_def_id)
        && let ExprKind::MethodCall(path, _trim_recv, [], trim_span) = recv.kind
        && let trim_fn_name @ ("trim" | "trim_start" | "trim_end") = path.ident.name.as_str()
        && let Some(trim_def_id) = tyckres.type_dependent_def_id(recv.hir_id)
        && is_one_of_trim_diagnostic_items(cx, trim_def_id)
    {
        span_lint_and_sugg(
            cx,
            TRIM_SPLIT_WHITESPACE,
            trim_span.with_hi(call_span.lo()),
            format!("found call to `str::{trim_fn_name}` before `str::split_whitespace`"),
            format!("remove `{trim_fn_name}()`"),
            String::new(),
            Applicability::MachineApplicable,
        );
    }
}

fn is_one_of_trim_diagnostic_items(cx: &LateContext<'_>, trim_def_id: rustc_hir::def_id::DefId) -> bool {
    cx.tcx.is_diagnostic_item(sym::str_trim, trim_def_id)
        || cx.tcx.is_diagnostic_item(sym::str_trim_start, trim_def_id)
        || cx.tcx.is_diagnostic_item(sym::str_trim_end, trim_def_id)
}

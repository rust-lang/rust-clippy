use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{is_res_lang_ctor, path_res};
use rustc_errors::Applicability;
use rustc_hir::LangItem::{ResultErr, ResultOk};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;
use rustc_span::symbol::sym;

use super::OPTION_MAP_OR_ERR_OK;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'_>,
    or_expr: &'tcx Expr<'_>,
    map_expr: &'tcx Expr<'_>,
    map_or_span: Span,
) {
    // We check that it's called on an `Option` type.
    if is_type_diagnostic_item(cx, cx.typeck_results().expr_ty_adjusted(recv), sym::Option)
        // We check that first we pass an `Err`.
        && let ExprKind::Call(call, &[arg]) = or_expr.kind
        && is_res_lang_ctor(cx, path_res(cx, call), ResultErr)
        // And finally we check that it is mapped as `Ok`.
        && is_res_lang_ctor(cx, path_res(cx, map_expr), ResultOk)
    {
        let mut app = Applicability::MachineApplicable;
        let err_snippet = snippet_with_applicability(cx, arg.span, "_", &mut app);
        span_lint_and_then(
            cx,
            OPTION_MAP_OR_ERR_OK,
            expr.span,
            "called `map_or(Err(_), Ok)` on an `Option` value",
            |diag| {
                diag.span_suggestion_verbose(
                    map_or_span.with_hi(expr.span.hi()),
                    "consider using `ok_or`",
                    format!("ok_or({err_snippet})"),
                    app,
                );
            },
        );
    }
}

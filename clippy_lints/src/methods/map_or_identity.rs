use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_expr_identity_function;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;

use rustc_lint::LateContext;
use rustc_span::symbol::sym;

use super::MAP_OR_IDENTITY;

/// lint use of `_.map_or(err, |n| n)` for `Result`s and `Option`s.
pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, def_arg: &Expr<'_>, map_arg: &Expr<'_>) {
    // lint if the caller of `map_or()` is a `Result` or an `Option`
    // and if the mapping function is the identity function
    if let Some(symbol @ (sym::Result | sym::Option)) = cx.typeck_results().expr_ty(recv).opt_diag_name(cx)
        && is_expr_identity_function(cx, map_arg)
    {
        let msg = format!("expression can be simplified using `{symbol}::unwrap_or()`");
        span_lint_and_then(cx, MAP_OR_IDENTITY, expr.span, msg, |diag| {
            let mut applicability = Applicability::MachineApplicable;
            let self_snippet = snippet_with_applicability(cx, recv.span, "_", &mut applicability);
            let err_snippet = snippet_with_applicability(cx, def_arg.span, "..", &mut applicability);
            let sugg = format!("{self_snippet}.unwrap_or({err_snippet})");

            diag.span_suggestion(expr.span, "consider using `unwrap_or`", sugg, applicability);
        });
    }
}

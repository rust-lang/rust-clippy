use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_expr_identity_function;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;

use rustc_lint::LateContext;
use rustc_span::Symbol;
use rustc_span::symbol::sym;

use super::{UNNECESSARY_OPTION_MAP_OR_ELSE, UNNECESSARY_RESULT_MAP_OR_ELSE};

fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, def_arg: &Expr<'_>, symbol: Symbol) {
    let msg = format!("unused \"map closure\" when calling `{symbol}::map_or_else` value");
    let mut applicability = Applicability::MachineApplicable;
    let self_snippet = snippet_with_applicability(cx, recv.span, "_", &mut applicability);
    let err_snippet = snippet_with_applicability(cx, def_arg.span, "..", &mut applicability);
    span_lint_and_sugg(
        cx,
        match symbol {
            sym::Option => UNNECESSARY_OPTION_MAP_OR_ELSE,
            sym::Result => UNNECESSARY_RESULT_MAP_OR_ELSE,
            _ => panic!("This shouldn't happen"),
        },
        expr.span,
        msg,
        "consider using `unwrap_or_else`",
        format!("{self_snippet}.unwrap_or_else({err_snippet})"),
        applicability,
    );
}

// Option
// fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, def_arg: &Expr<'_>) {
//     let msg = "unused \"map closure\" when calling `Option::map_or_else` value";
//     let mut applicability = Applicability::MachineApplicable;
//     let self_snippet = snippet_with_applicability(cx, recv.span, "_", &mut applicability);
//     let err_snippet = snippet_with_applicability(cx, def_arg.span, "..", &mut applicability);
//     span_lint_and_sugg(
//         cx,
//         UNNECESSARY_OPTION_MAP_OR_ELSE,
//         expr.span,
//         msg,
//         "consider using `unwrap_or_else`",
//         format!("{self_snippet}.unwrap_or_else({err_snippet})"),
//         Applicability::MachineApplicable,
//     );
// }

/// lint use of `_.map_or_else(|err| err, |n| n)` for `Result`s.
pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, def_arg: &Expr<'_>, map_arg: &Expr<'_>) {
    // lint if the caller of `map_or_else()` is a `Result`
    if let Some(x @ (sym::Result | sym::Option)) = cx.typeck_results().expr_ty(recv).opt_diag_name(cx)
        && is_expr_identity_function(cx, map_arg)
    {
        emit_lint(cx, expr, recv, def_arg, x);
    }
}

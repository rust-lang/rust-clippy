use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_ast::ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::symbol::sym::Path;

use super::JOIN_ABSOLUTE_PATHS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, join_arg: &'tcx Expr<'tcx>) {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    if is_type_diagnostic_item(cx, ty, Path)
        && let ExprKind::Lit(spanned) = &join_arg.kind
        && let LitKind::Str(symbol, _) = spanned.node
        && (symbol.as_str().starts_with('/') || symbol.as_str().starts_with('\\'))
    {
        span_lint_and_then(
            cx,
            JOIN_ABSOLUTE_PATHS,
            join_arg.span,
            "argument to `Path::join` starts with a path separator",
            |diag| {
                diag
                      .note("joining a path starting with separator will replace the path instead")
                      .help(r#"if this is unintentional, try removing the starting separator"#)
                      .help(r#"if this is intentional, try creating a new Path instead"#);
            },
        );
    }
}

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::Expr;
use rustc_lint::LateContext;

use super::MAP_THEN_IDENTITY_TRANSFORMER;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    span_lint_and_help(
        cx,
        MAP_THEN_IDENTITY_TRANSFORMER,
        expr.span,
        &format!("map_all"),
        None,
        &format!("map_all"),
    );
}

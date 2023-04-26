use clippy_utils::{diagnostics::span_lint_and_sugg, ty::is_type_diagnostic_item};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::{symbol::sym::Path, Span};

use super::PATH_JOIN_CORRECTION;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, join_arg: &'tcx Expr<'tcx>, span: Span) {
    let ty = cx.typeck_results().expr_ty(expr);
    if_chain!(
    if is_type_diagnostic_item(cx, ty, Path);
    let applicability = Applicability::MachineApplicable;
    if let ExprKind::Lit(spanned) = &join_arg.kind;
    if let LitKind::Str(symbol, _) = spanned.node;
    if symbol.as_str().starts_with('/') || symbol.as_str().starts_with('\\');
     then {
      span_lint_and_sugg(
      cx,
      PATH_JOIN_CORRECTION,
      span.with_hi(expr.span.hi()),
           r#"argument in join called on path contains a starting '/'"#,
           "try removing first '/' or '\\'",
           "join(\"your/path/here\")".to_owned(),
           applicability
         );
        }
        );
}

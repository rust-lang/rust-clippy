use clippy_macros::expr_sugg;
use clippy_utils::_internal::lint_expr_and_sugg;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::BYTES_NTH;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>, n_arg: &'tcx Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();
    let caller_type = if ty.is_str() {
        "str"
    } else if is_type_diagnostic_item(cx, ty, sym::String) {
        "String"
    } else {
        return;
    };
    lint_expr_and_sugg(
        cx,
        BYTES_NTH,
        &format!("called `.bytes().nth()` on a `{}`", caller_type),
        expr,
        "try",
        expr_sugg!({}.as_bytes().get({}), recv, n_arg),
        Applicability::MachineApplicable,
    );
}

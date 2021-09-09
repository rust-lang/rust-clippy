use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_item;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::BYTES_NTH;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, recv: &'tcx Expr<'tcx>, n_arg: &'tcx Expr<'tcx>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();
    let caller_type = if ty.is_str() {
        "str"
    } else if is_item(cx, ty, sym::string_type) {
        "String"
    } else {
        return;
    };
    let mut applicability = Applicability::MachineApplicable;
    span_lint_and_sugg(
        cx,
        BYTES_NTH,
        expr.span,
        &format!("called `.byte().nth()` on a `{}`", caller_type),
        "try",
        format!(
            "{}.as_bytes().get({})",
            snippet_with_applicability(cx, recv.span, "..", &mut applicability),
            snippet_with_applicability(cx, n_arg.span, "..", &mut applicability)
        ),
        applicability,
    );
}

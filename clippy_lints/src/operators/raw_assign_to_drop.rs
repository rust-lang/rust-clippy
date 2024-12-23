use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::LateContext;

use super::RAW_ASSIGN_TO_DROP;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, lhs: &'tcx Expr<'_>) {
    if let ExprKind::Unary(UnOp::Deref, expr) = lhs.kind
        && let ty = cx.typeck_results().expr_ty(expr)
        && ty.is_unsafe_ptr()
        && let Some(deref_ty) = ty.builtin_deref(true)
        && deref_ty.needs_drop(cx.tcx, cx.typing_env())
    {
        span_lint_and_then(
            cx,
            RAW_ASSIGN_TO_DROP,
            expr.span,
            "assignment via raw pointer always executes destructor",
            |diag| {
                diag.note(format!(
                    "the destructor defined by `{deref_ty}` is executed during assignment of the new value"
                ));
                diag.span_label(
                    expr.span,
                    "this place may be uninitialized, causing Undefined Behavior when the destructor executes",
                );
                diag.help("use `std::ptr::write()` to overwrite a (possibly uninitialized) place");
                diag.help("use `std::ptr::drop_in_place()` to drop the previous value if such value exists");
            },
        );
    }
}

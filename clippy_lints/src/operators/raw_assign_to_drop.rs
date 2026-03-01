use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::LateContext;

use super::RAW_ASSIGN_TO_DROP;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, lhs: &'tcx Expr<'_>) {
    if let ExprKind::Unary(UnOp::Deref, expr) = lhs.kind
        && let ty = cx.typeck_results().expr_ty(expr)
        && ty.is_raw_ptr()
        && let Some(deref_ty) = ty.builtin_deref(true)
        && deref_ty.needs_drop(cx.tcx, cx.typing_env())
    {
        if let ExprKind::MethodCall(path, self_arg, [], ..) = expr.kind
            && let rustc_middle::ty::Adt(ty_def, ..) = cx.typeck_results().expr_ty(self_arg).kind()
            && ty_def.is_unsafe_cell()
            && path.ident.as_str() == "get"
        {
            // Don't lint if the raw pointer was directly retrieved from UnsafeCell::get()
            // We assume those to be safely managed
            return;
        }
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
                    "the old value may be uninitialized, causing Undefined Behavior when the destructor executes",
                );
                diag.help("use `std::ptr::write()` to overwrite a possibly uninitialized place");
                diag.help(
                    "use `std::ptr::drop_in_place()` to drop the previous value, having established such value exists",
                );
            },
        );
    }
}

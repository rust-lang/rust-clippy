use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef;
use rustc_hir::{Expr, ExprKind, LangItem, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, func: &Expr<'_>, args: &[Expr<'_>]) {
    if let [arg] = args
        && let ExprKind::Path(QPath::Resolved(_, path)) = func.kind
        && let Some(def_id) = path.res.opt_def_id()
        && cx.tcx.lang_items().drop_in_place_fn() == Some(def_id)
        && let ty::RawPtr(mut pointee, _) = *cx.typeck_results().expr_ty_adjusted(arg).kind()
    {
        // See through `[MaybeUninit<T>]` and `MaybeUninit<[T; N]>`
        let mut is_slice_or_array = false;
        while let ty::Array(inner, _) | ty::Slice(inner) = *pointee.kind() {
            pointee = inner;
            is_slice_or_array = true;
        }

        if !pointee.is_lang_item(cx, LangItem::MaybeUninit) {
            return;
        }

        // Intentionally no structured suggestion, since we don't know if the `MaybeUninit`
        // is initialized or not in the user's code.
        span_lint_and_then(
            cx,
            super::MAYBE_UNINIT_DROP_IN_PLACE,
            expr.span,
            "calling `ptr::drop_in_place` on a `MaybeUninit` is a no-op",
            |diag| {
                diag.note("`MaybeUninit<T>` does not implement `Drop`, so the wrapped `T` is not dropped");
                if is_slice_or_array {
                    diag.help(
                        "if initialized, drop the inner `T`s by casting the pointer to `*mut T` before forming the slice",
                    );
                } else {
                    diag.help("if initialized, drop the inner `T` with `MaybeUninit::assume_init_drop`");
                }
            },
        );
    }
}

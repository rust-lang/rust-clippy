use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::{is_full_collection_range, sym};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::ITER_WITH_DRAIN;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, span: Span, arg: &Expr<'_>) {
    if let Some(adt) = cx.typeck_results().expr_ty(recv).ty_adt_def()
        && let Some(ty_name) = cx.tcx.get_diagnostic_name(adt.did())
        && matches!(ty_name, sym::Vec | sym::VecDeque)
        && is_full_collection_range(cx, recv.res_local_id(), arg)
        && can_move_receiver(cx, expr, recv)
    {
        span_lint_and_sugg(
            cx,
            ITER_WITH_DRAIN,
            span.with_hi(expr.span.hi()),
            format!("`drain(..)` used on a `{ty_name}`"),
            "try",
            "into_iter()".to_string(),
            Applicability::MaybeIncorrect,
        );
    }
}

/// `into_iter()` takes the collection by value, so the suggestion only compiles when the receiver
/// is either a temporary or a local that is never used again (including on later iterations of an
/// enclosing loop or calls of an enclosing closure).
fn can_move_receiver(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) -> bool {
    match recv.res_local_id() {
        Some(local_id) => !local_used_after_expr(cx, local_id, expr),
        // Fields, derefs, indexes and statics can't be moved out of.
        None => !recv.is_syntactic_place_expr(),
    }
}

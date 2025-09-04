use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_custom_deref_adjustment;
use clippy_utils::res::MaybeDef;
use clippy_utils::ty::peel_and_count_ty_refs;
use rustc_errors::Applicability;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    if matches!(expr_custom_deref_adjustment(cx, recv), None | Some(Mutability::Mut))
        // NOTE: the reason we don't use `expr_ty_adjusted` here is that a call to `Mutex::lock` by itself
        // adjusts the receiver to be `&Mutex`
        && let (recv_ty, _, Some(Mutability::Mut)) = peel_and_count_ty_refs(cx.typeck_results().expr_ty(recv))
        && recv_ty.is_diag_item(cx, sym::Mutex)
    {
        span_lint_and_sugg(
            cx,
            MUT_MUTEX_LOCK,
            name_span,
            "calling `&mut Mutex::lock` unnecessarily locks an exclusive (mutable) reference",
            "change this to",
            "get_mut".to_owned(),
            Applicability::MaybeIncorrect,
        );
    }
}

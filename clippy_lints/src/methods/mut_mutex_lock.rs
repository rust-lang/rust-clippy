use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::ty::peel_and_count_ty_refs;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    // We don't want to lint on indexing and field accesses, as both of those would take exclusive
    // access to only part of a value -- which would conflict with any immutable reborrow over
    // the whole value
    if !matches!(recv.kind, ExprKind::Field(..) | ExprKind::Index(..))
        // Make sure that we have a mutable access to `Mutex`, while not outright owning it -- if
        // the latter were the case, then changing `.lock()` to `.get_mut()`, could easily result in
        // a conflict with other existing immutable borrows. Note that `mutbl` being `Some` is
        // enough to determine that there was at least one layer of references
        //
        // NOTE: the reason we don't use `expr_ty_adjusted` here is that a call to `Mutex::lock` by
        // itself adjusts the receiver to be `&Mutex`
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

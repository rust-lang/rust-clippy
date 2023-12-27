use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_custom_deref_adjustment;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{sym, Span};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, ex: &'tcx Expr<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    if matches!(expr_custom_deref_adjustment(cx, recv), None | Some(Mutability::Mut))
        && let ty::Ref(_, _, Mutability::Mut) = cx.typeck_results().expr_ty(recv).kind()
        && let Some(method_id) = cx.typeck_results().type_dependent_def_id(ex.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
        && is_type_diagnostic_item(cx, cx.tcx.type_of(impl_id).instantiate_identity(), sym::Mutex)
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

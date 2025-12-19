use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::Adjust;
use rustc_span::{Span, sym};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    let typeck = cx.typeck_results();
    if !typeck.expr_ty_adjusted(recv).peel_refs().is_diag_item(cx, sym::Mutex) {
        return;
    }

    let deref_mut_trait = cx.tcx.lang_items().deref_mut_trait();
    let index_mut_trait = cx.tcx.lang_items().index_mut_trait();
    let impls_deref_mut = |ty| deref_mut_trait.is_some_and(|trait_id| implements_trait(cx, ty, trait_id, &[]));
    let impls_index_mut = |ty, idx| index_mut_trait.is_some_and(|trait_id| implements_trait(cx, ty, trait_id, &[idx]));
    let mut r = recv;
    loop {
        if (typeck.expr_adjustments(r))
            .iter()
            .map_while(|a| match a.kind {
                Adjust::Deref(x) => Some((a.target, x)),
                _ => None,
            })
            .try_fold(typeck.expr_ty(r), |ty, (target, deref)| match deref {
                Some(_) => impls_deref_mut(ty).then_some(target),
                None => (ty.ref_mutability() != Some(Mutability::Not)).then_some(target),
            })
            .is_none()
        {
            return;
        }
        match r.kind {
            ExprKind::Field(base, _) => r = base,
            ExprKind::Index(base, idx, _) => {
                if impls_index_mut(typeck.expr_ty_adjusted(base), typeck.expr_ty_adjusted(idx).into()) {
                    r = base;
                } else {
                    return;
                }
            },
            ExprKind::Unary(UnOp::Deref, base) => {
                if impls_deref_mut(typeck.expr_ty_adjusted(base)) {
                    r = base;
                } else {
                    return;
                }
            },
            _ => break,
        }
    }

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

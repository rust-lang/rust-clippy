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

    // The mutex was accessed either directly (`mutex.lock()`), or through a series of
    // deref/field/indexing projections. Since the final `.lock()` call only requires `&Mutex`,
    // those might be immutable, and so we need to manually check whether mutable projections
    // would've been possible. For that, we'll repeatedly peel off projections and check each
    // intermediary receiver.
    let mut r = recv;
    loop {
        // Check that the (deref) adjustments could be made mutable
        if (typeck.expr_adjustments(r))
            .iter()
            .map_while(|a| match a.kind {
                Adjust::Deref(x) => Some((a.target, x)),
                _ => None,
            })
            .try_fold(typeck.expr_ty(r), |ty, (target, deref)| match deref {
                // There has been an overloaded deref, most likely an immutable one, as `.lock()` didn't require a
                // mutable one -- we need to check if a mutable deref would've been possible, i.e. if
                // `ty: DerefMut<Target = target>` (we don't need to check the `Target` part, as `Deref` and `DerefMut`
                // impls necessarily have the same one)
                Some(_) => impls_deref_mut(ty).then_some(target),
                // There has been a simple deref; if it happened on a `&T`, then we know it will can't be changed to
                // provide mutable access
                None => (ty.ref_mutability() != Some(Mutability::Not)).then_some(target),
            })
            .is_none()
        {
            return;
        }

        // Peel off one projection
        match r.kind {
            // In order to be able to make this `*` mean `.deref_mut()`, `base: DerefMut` needs to hold
            ExprKind::Unary(UnOp::Deref, base) => {
                if impls_deref_mut(typeck.expr_ty_adjusted(base)) {
                    r = base;
                } else {
                    return;
                }
            },
            // In order to be able to make this `[idx]` mean `.index_mut(idx)`, `base: IndexMut<idx>` needs to hold
            ExprKind::Index(base, idx, _) => {
                // NOTE: the reason we do need to take into account `idx` here is that it's a _generic_ of
                // `IndexMut`, not an associated type of the impl
                if impls_index_mut(typeck.expr_ty_adjusted(base), typeck.expr_ty_adjusted(idx).into()) {
                    r = base;
                } else {
                    return;
                }
            },
            // A field projection by itself can't be mutable/immutable -- we'll only need to check
            // that the field type is not a `&T`, and we'll do that in the next iteration of the
            // loop, during adjustment checking
            ExprKind::Field(base, _) => r = base,
            // We arrived at the innermost receiver
            _ => {
                if let ExprKind::Path(ref p) = r.kind
                    && cx
                        .qpath_res(p, r.hir_id)
                        .opt_def_id()
                        .and_then(|id| cx.tcx.static_mutability(id))
                        == Some(Mutability::Not)
                {
                    // The mutex is stored in a `static`, and we don't want to suggest making that
                    // mutable
                    return;
                }
                // No more projections to check
                break;
            },
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

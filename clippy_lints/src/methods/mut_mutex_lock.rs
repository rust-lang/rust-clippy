use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_custom_deref_adjustment;
use clippy_utils::res::MaybeDef;
use clippy_utils::ty::{implements_trait, peel_and_count_ty_refs};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
use rustc_span::{Span, sym};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    let typeck = cx.typeck_results();

    // Make sure that we have a mutable access to `Mutex`:
    //
    // 1. Check that there are no custom deref adjustments turning the access immutable (e.g.
    //    `Arc<Mutex>` derefs to `&Mutex`)
    if matches!(expr_custom_deref_adjustment(cx, recv), None | Some(Mutability::Mut))
        // 2. Check that the receiver is behind one or more `&mut`s -- we want to have mutable
        //    access, while not outright owning it -- if the latter were the case, then changing
        //    `.lock()` to `.get_mut()`, could easily result in a conflict with other existing
        //    immutable borrows.
        //
        // NOTE: `mutbl` being `Some` is enough to determine that there was at least one layer of
        // references, so no need to check `n`
        // NOTE: the reason we don't use `expr_ty_adjusted` here is that a call
        // to `Mutex::lock` by itself adjusts the receiver to be `&Mutex`
        && let (recv_ty, _, Some(Mutability::Mut)) = peel_and_count_ty_refs(typeck.expr_ty(recv))
        && recv_ty.is_diag_item(cx, sym::Mutex)
    {
        let deref_mut_trait = cx.tcx.lang_items().deref_mut_trait();
        let impls_deref_mut = |ty| deref_mut_trait.is_some_and(|trait_id| implements_trait(cx, ty, trait_id, &[]));

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
                    // `ty: DerefMut<Target = target>` (we don't need to check the `Target` part, as `Deref` and
                    // `DerefMut` impls necessarily have the same one)
                    DerefAdjustKind::Overloaded(_) => impls_deref_mut(ty).then_some(target),
                    // There has been a simple deref; if it happened on a `&T`, then we know it will can't be changed to
                    // provide mutable access
                    DerefAdjustKind::Builtin => (ty.ref_mutability() != Some(Mutability::Not)).then_some(target),
                    DerefAdjustKind::Pin => Some(target),
                })
                .is_none()
            {
                return;
            }

            // Peel off one projection
            match r.kind {
                ExprKind::Unary(UnOp::Deref, base) => {
                    if impls_deref_mut(typeck.expr_ty_adjusted(base)) {
                        r = base;
                    } else {
                        return;
                    }
                },
                ExprKind::Index(..) | ExprKind::Field(..) => {
                    // We don't want to lint on indexing and field accesses, as both of those would take exclusive
                    // access to only part of a value -- which would conflict with any immutable reborrow over
                    // the whole value
                    return;
                },
                _ => {
                    // We arrived at the innermost receiver
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
}

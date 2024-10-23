use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_path_mutable;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::{deref_chain, get_inherent_method, implements_trait, make_normalized_projection};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

use super::{UNNECESSARY_COLLECTION_CLONE, method_call};

// FIXME: This does not check if the iter method is actually compatible with the replacement, but
// you have to be actively evil to have an `IntoIterator` impl that returns one type and an `iter`
// method that returns something other than references of that type.... and it is a massive
// complicated hassle to check this
fn has_iter_method<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    deref_chain(cx, ty).any(|ty| match ty.kind() {
        ty::Adt(adt_def, _) => get_inherent_method(cx, adt_def.did(), sym::iter).is_some(),
        ty::Slice(_) => true,
        _ => false,
    })
}

/// Check for `x.clone().into_iter()` to suggest `x.iter().cloned()`.
//             ^^^^^^^^^ is recv
//             ^^^^^^^^^^^^^^^^^^^^^ is expr
pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) {
    let typeck_results = cx.typeck_results();
    let diagnostic_items = cx.tcx.all_diagnostic_items(());

    // If the call before `into_iter` is `.clone()`
    if let Some(("clone", collection_expr, [], _, _)) = method_call(recv)
        // and the binding being cloned is not mutable
        && let Some(false) = is_path_mutable(cx, collection_expr)
        // and the result of `into_iter` is an Iterator
        && let Some(&iterator_def_id) = diagnostic_items.name_to_id.get(&sym::Iterator)
        && let expr_ty = typeck_results.expr_ty(expr)
        && implements_trait(cx, expr_ty, iterator_def_id, &[])
        // with an item that implements clone
        && let Some(&clone_def_id) = diagnostic_items.name_to_id.get(&sym::Clone)
        && let Some(item_ty) = make_normalized_projection(cx.tcx, cx.param_env, iterator_def_id, sym::Item, [expr_ty])
        && implements_trait(cx, item_ty, clone_def_id, &[])
        // and the type has an `iter` method
        && has_iter_method(cx, typeck_results.expr_ty(collection_expr))
    {
        let mut applicability = Applicability::MachineApplicable;
        let collection_expr_snippet = snippet_with_applicability(cx, collection_expr.span, "...", &mut applicability);
        span_lint_and_sugg(
            cx,
            UNNECESSARY_COLLECTION_CLONE,
            expr.span,
            "using clone on collection to own iterated items",
            "replace with",
            format!("{collection_expr_snippet}.iter().cloned()"),
            applicability,
        );
    }
}

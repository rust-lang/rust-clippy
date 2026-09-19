use clippy_utils::diagnostics::span_lint_hir_and_then;
use rustc_hir::{HirId, TraitRef};
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;
use rustc_span::{Span, sym};

use super::DERIVED_PARTIAL_EQ_WITH_MANUAL_EQ;

/// Implementation of the `DERIVED_PARTIAL_EQ_WITH_MANUAL_EQ` lint.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    span: Span,
    trait_ref: &TraitRef<'_>,
    ty: Ty<'tcx>,
    adt_hir_id: HirId,
) {
    if let Some(peq_trait_def_id) = cx.tcx.lang_items().eq_trait()
        && let Some(eq_trait_def_id) = cx.tcx.get_diagnostic_item(sym::Eq)
        && let Some(def_id) = trait_ref.trait_def_id()
        && def_id == peq_trait_def_id
    {
        // Look for the PartialEq implementations for `ty`
        cx.tcx.for_each_relevant_impl(peq_trait_def_id, ty, |peq_impl_id| {
            let peq_is_automatically_derived = cx.tcx.is_automatically_derived(peq_impl_id);
            let peq_trait_ref = cx.tcx.impl_trait_ref(peq_impl_id);

            // Look for the Eq implementations for `ty`
            cx.tcx.for_each_relevant_impl(eq_trait_def_id, ty, |eq_impl_id| {
                let eq_is_automatically_derived = cx.tcx.is_automatically_derived(eq_impl_id);
                let eq_trait_ref = cx.tcx.impl_trait_ref(eq_impl_id);

                // Only care about manually implementing Eq and PartialEq is automatically derived.
                if peq_is_automatically_derived
                    && !eq_is_automatically_derived
                    // Only care about `impl PartialEq<Foo> for Foo`
                    // For `impl PartialEq<B> for A, input_types is [A, B]
                    && peq_trait_ref.instantiate_identity().skip_norm_wip().args.type_at(1) == ty
                    // `for_each_relevant_impl` only does a coarse "could possibly match" filter,
                    // so it can return impls unrelated to `ty` (e.g. blanket impls like
                    // `core::ptr`'s `Eq` impl). Confirm both impls are actually for `ty` itself.
                    && eq_trait_ref.instantiate_identity().skip_norm_wip().args.type_at(0) == ty
                {
                    span_lint_hir_and_then(
                        cx,
                        DERIVED_PARTIAL_EQ_WITH_MANUAL_EQ,
                        adt_hir_id,
                        span,
                        "when `PartialEq` is derived, `Eq` should be derived too",
                        |diag| {
                            if let Some(eq_local_def_id) = eq_impl_id.as_local() {
                                let hir_id = cx.tcx.local_def_id_to_hir_id(eq_local_def_id);
                                diag.span_label(cx.tcx.hir_span(hir_id), "manual implementation here");
                            }
                            if let Some(peq_local_def_id) = peq_impl_id.as_local() {
                                let hir_id = cx.tcx.local_def_id_to_hir_id(peq_local_def_id);
                                diag.span_label(cx.tcx.hir_span(hir_id), "`PartialEq` is derived here");
                            }
                        },
                    );
                }
            });
        });
    }
}

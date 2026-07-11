use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

use super::{DEREF_METHOD_CALL_CHAIN, method_call};

/// Checks whether the receiver of `outer_name` is a redundant deref-like conversion call
/// (e.g. `Vec::as_slice`, `String::as_str`) which can be removed because method resolution
/// would find the same method on the original value through deref coercion.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    outer_name: Symbol,
    outer_span: Span,
    recv: &'tcx Expr<'tcx>,
) {
    let Some((inner_name, inner_recv, [], inner_span, _)) = method_call(recv) else {
        return;
    };
    if !matches!(
        inner_name,
        sym::as_c_str
            | sym::as_mut_slice
            | sym::as_mut_str
            | sym::as_os_str
            | sym::as_path
            | sym::as_slice
            | sym::as_str
    ) {
        return;
    }

    let Some(inner_did) = cx.typeck_results().type_dependent_def_id(recv.hir_id) else {
        return;
    };
    let inner_diag = cx.tcx.get_diagnostic_name(inner_did);
    let is_std_conversion = matches!(
        inner_diag,
        Some(
            sym::cstring_as_c_str
                | sym::os_string_as_os_str
                | sym::pathbuf_as_path
                | sym::string_as_mut_str
                | sym::string_as_str
                | sym::vec_as_mut_slice
                | sym::vec_as_slice
        )
    );

    // `String::as_str()` followed by these methods is covered by `REDUNDANT_AS_STR`
    if inner_diag == Some(sym::string_as_str) && matches!(outer_name, sym::as_bytes | sym::is_empty) {
        return;
    }

    // The conversion must be deref-equivalent: it returns a reference to exactly
    // `<RecvTy as Deref>::Target`, so removing it lets auto-deref do the same work
    let recv_base_ty = cx.typeck_results().expr_ty(inner_recv).peel_refs();
    let ty::Ref(_, target_ty, _) = *cx.typeck_results().expr_ty(recv).kind() else {
        return;
    };
    let Some(deref_trait_id) = cx.tcx.get_diagnostic_item(sym::Deref) else {
        return;
    };
    if !implements_trait(cx, recv_base_ty, deref_trait_id, &[])
        || cx.get_associated_type(recv_base_ty, deref_trait_id, sym::Target) != Some(target_ty)
    {
        return;
    }

    // The following method must be an inherent method of the deref target. Trait methods
    // could resolve to a different impl on the original receiver (e.g. `IntoIterator` on
    // `Vec<T>` vs `&[T]`), changing semantics
    let Some(outer_did) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return;
    };
    let Some(impl_id) = cx.tcx.inherent_impl_of_assoc(outer_did) else {
        return;
    };
    let outer_args = cx.typeck_results().node_args(expr.hir_id);
    let impl_args = outer_args.truncate_to(cx.tcx, cx.tcx.generics_of(impl_id));
    if cx.tcx.type_of(impl_id).instantiate(cx.tcx, impl_args).skip_norm_wip() != target_ty {
        return;
    }

    // For user-defined conversions, a same-named inherent method on the receiver type would
    // shadow the target's method. Std receivers (`Vec`, `String`, ...) only shadow with
    // delegating methods (`Vec::len`, ...), which are exactly the idiomatic suggestion
    if !is_std_conversion
        && let Some(adt) = recv_base_ty.ty_adt_def()
        && cx
            .tcx
            .inherent_impls(adt.did())
            .iter()
            .flat_map(|&impl_id| cx.tcx.associated_items(impl_id).filter_by_name_unhygienic(outer_name))
            .any(ty::AssocItem::is_method)
    {
        return;
    }

    let mut applicability = Applicability::MachineApplicable;
    span_lint_and_sugg(
        cx,
        DEREF_METHOD_CALL_CHAIN,
        inner_span.to(outer_span),
        format!("this `{inner_name}` call is redundant as `{outer_name}` can be called on the receiver directly"),
        "call the method directly",
        snippet_with_applicability(cx, outer_span, "..", &mut applicability).into_owned(),
        applicability,
    );
}

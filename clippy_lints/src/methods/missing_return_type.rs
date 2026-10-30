use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::return_ty;
use rustc_hir::{FnSig, ImplItem, ImplItemKind, TraitItem, TraitItemKind};
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;
use rustc_span::Ident;

use super::MISSING_RETURN_TYPE;

pub(super) fn check_trait_item(cx: &LateContext<'_>, trait_item: &TraitItem<'_>) {
    if let TraitItemKind::Fn(sig, _) = trait_item.kind {
        check(cx, trait_item.ident, return_ty(cx, trait_item.owner_id), sig);
    }
}

pub(super) fn check_impl_item(cx: &LateContext<'_>, impl_item: &ImplItem<'_>, implements_trait: bool) {
    if !implements_trait && let ImplItemKind::Fn(sig, _) = impl_item.kind {
        check(cx, impl_item.ident, return_ty(cx, impl_item.owner_id), sig);
    }
}

fn check(cx: &LateContext<'_>, ident: Ident, ret_ty: Ty<'_>, sig: FnSig<'_>) {
    let method_name = ident.name.as_str();
    if (method_name.starts_with("as_") || method_name.starts_with("to_") || method_name.starts_with("into_"))
        && ret_ty.is_unit()
    {
        span_lint_and_help(
            cx,
            MISSING_RETURN_TYPE,
            sig.decl.output.span(),
            "conversion does not return with any type",
            None,
            "rename the method or return a type",
        );
    }
}

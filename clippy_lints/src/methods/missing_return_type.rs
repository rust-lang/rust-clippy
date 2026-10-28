use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::return_ty;
use rustc_hir::{ImplItem, ImplItemKind};
use rustc_lint::LateContext;

use super::MISSING_RETURN_TYPE;

pub(super) fn check(cx: &LateContext<'_>, impl_item: &ImplItem<'_>) {
    let method_name = impl_item.ident.name.as_str();
    if let ImplItemKind::Fn(sig, _body) = impl_item.kind
        && (method_name.starts_with("as_") || method_name.starts_with("to_") || method_name.starts_with("into_"))
        && return_ty(cx, impl_item.owner_id).is_unit()
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

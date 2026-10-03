use super::{DOC_EXAMPLES_MISSING_ITEM, is_public_api};
use clippy_utils::attrs::is_proc_macro;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_lint_allowed, is_trait_impl_item, tokenize_with_text};
use rustc_hir::def::DefKind;
use rustc_lexer::TokenKind;
use rustc_lint::LateContext;
use rustc_span::Ident;

pub(super) fn item_ident(cx: &LateContext<'_>, check_private_items: bool) -> Option<Ident> {
    let hir_id = cx.last_node_with_lint_attrs;
    let owner_id = hir_id.as_owner()?;
    if !is_lint_allowed(cx, DOC_EXAMPLES_MISSING_ITEM, hir_id)
        // Types and traits are skipped because examples often use them through a glob import without naming
        // them, and telling which glob-imported names an example uses needs name resolution on every doctest.
        && matches!(
            cx.tcx.def_kind(owner_id),
            DefKind::Fn | DefKind::AssocFn | DefKind::Const | DefKind::AssocConst | DefKind::Static { .. }
        )
        && !is_trait_impl_item(cx, hir_id)
        && !is_proc_macro(cx.tcx.hir_attrs(hir_id))
        && (check_private_items || is_public_api(cx, owner_id))
    {
        cx.tcx.opt_item_ident(owner_id.to_def_id())
    } else {
        None
    }
}

pub(super) fn mentions(code: &str, ident: Ident) -> bool {
    if !code.contains(ident.as_str()) {
        return false;
    }

    tokenize_with_text(code).any(|(kind, text, _)| {
        matches!(kind, TokenKind::Ident | TokenKind::RawIdent)
            && text.strip_prefix("r#").unwrap_or(text) == ident.as_str()
    })
}

pub(super) fn report(cx: &LateContext<'_>, ident: Ident) {
    span_lint_and_help(
        cx,
        DOC_EXAMPLES_MISSING_ITEM,
        cx.tcx.def_span(cx.last_node_with_lint_attrs.owner),
        format!("none of the documentation examples mention `{ident}`"),
        None,
        "consider adding an example that uses this item",
    );
}

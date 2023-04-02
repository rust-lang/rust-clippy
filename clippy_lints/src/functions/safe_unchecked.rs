use clippy_utils::{diagnostics::span_lint_and_then, is_trait_impl_item};
use rustc_errors::Applicability;
use rustc_hir::{intravisit::FnKind, HirId};
use rustc_lint::LateContext;
use rustc_span::{symbol::Ident, Span};

use super::SAFE_UNCHECKED;

pub(super) fn check_fn(cx: &LateContext<'_>, kind: &FnKind<'_>, sp: Span, hir_id: HirId) {
    fn lint_if_name_ends_with_unchecked(cx: &LateContext<'_>, item_span: Span, ident: &Ident) {
        if ident.as_str().ends_with("_unchecked") {
            span_lint_and_then(
                cx,
                SAFE_UNCHECKED,
                ident.span,
                "this function ending with `_unchecked` isn't marked with `unsafe`",
                |diag| {
                    diag.span_suggestion_verbose(
                        item_span.shrink_to_lo().with_lo(item_span.lo()),
                        "add an unsafe marker",
                        "unsafe ",
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }

    match kind {
        FnKind::ItemFn(ident, _, header) => {
            if !header.is_unsafe() {
                lint_if_name_ends_with_unchecked(cx, sp, ident);
            }
        },
        FnKind::Method(ident, sig) => {
            if !sig.header.is_unsafe() && !is_trait_impl_item(cx, hir_id) {
                lint_if_name_ends_with_unchecked(cx, sp, ident);
            }
        },
        FnKind::Closure => (),
    }
}

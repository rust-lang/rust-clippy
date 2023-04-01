use clippy_utils::{diagnostics::span_lint_and_then, is_trait_impl_item};
use rustc_errors::Applicability;
use rustc_hir::{intravisit::FnKind, HirId, TraitItem};
use rustc_lint::LateContext;
use rustc_span::{sym, symbol::Ident};

use super::IMPOLITE;

const POLITE_WORDS: [&str; 2] = ["please", "pls"];

fn please_check(cx: &LateContext<'_>, ident: Ident) {
    let name = ident.name.as_str();

    for word in &POLITE_WORDS {
        if name.starts_with(word) || name.ends_with(word) {
            return;
        }
    }

    let span = ident.span;
    let prefix_suggestion = format!("please_{name}");
    let suffix_suggestion = format!("{name}_please");

    span_lint_and_then(cx, IMPOLITE, span, "function name is impolite", |diag| {
        diag.span_suggestion(
            span,
            "consider using a polite prefix",
            prefix_suggestion,
            Applicability::MaybeIncorrect,
        );

        diag.span_suggestion(
            span,
            "consider using a polite suffix",
            suffix_suggestion,
            Applicability::MaybeIncorrect,
        );
    });
}

pub fn please_check_fn(cx: &LateContext<'_>, kind: FnKind<'_>, hir_id: HirId) {
    match kind {
        FnKind::ItemFn(ident, _, _) => {
            // Ignore main for now
            // TODO: Rename main to please_main in rustc
            if ident.name != sym::main {
                please_check(cx, ident);
            }
        },
        FnKind::Method(ident, _) => {
            // Ignore trait impls
            if !is_trait_impl_item(cx, hir_id) {
                please_check(cx, ident);
            }
        },
        FnKind::Closure => {},
    };
}

pub fn please_check_trait_item(cx: &LateContext<'_>, item: &TraitItem<'_>) {
    please_check(cx, item.ident);
}

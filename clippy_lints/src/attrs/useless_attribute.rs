use super::USELESS_ATTRIBUTE;
use super::utils::{is_lint_level, is_word, namespace_and_lint};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{SpanExt as _, first_line_of_span};
use clippy_utils::sym;
use rustc_ast::{Attribute, Item, ItemKind, MetaItemInner};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext as _};

/// Returns whether `lint` can be emitted on the `item` itself, in which case a lint level attribute
/// naming it is meaningful and must not be reported as useless.
fn is_emitted_on_item(item: &Item, lint: &MetaItemInner, skip_unused_imports: bool) -> bool {
    match item.kind {
        ItemKind::Use(..) => match namespace_and_lint(lint) {
            (Some(sym::clippy), Some(name)) => matches!(
                name,
                sym::wildcard_imports
                    | sym::enum_glob_use
                    | sym::redundant_pub_crate
                    | sym::macro_use_imports
                    | sym::unsafe_removed_from_name
                    | sym::module_name_repetitions
                    | sym::single_component_path_imports
                    | sym::disallowed_types
                    | sym::unused_trait_names
                    // Lints that fire on lint attributes themselves. They are commonly allowed on a
                    // `use` item to permit an attribute that is only useless for some macro
                    // expansions, see <https://github.com/rust-lang/rust-clippy/issues/17562>.
                    | sym::allow_attributes
                    | sym::useless_attribute
            ),
            (None, Some(name)) => matches!(
                name,
                sym::ambiguous_glob_reexports
                    | sym::dead_code
                    | sym::deprecated
                    | sym::deprecated_in_future
                    | sym::exported_private_dependencies
                    | sym::hidden_glob_reexports
                    | sym::unreachable_pub
                    | sym::unused
                    | sym::unused_braces
                    | sym::unused_import_braces
                    | sym::unused_imports
                    | sym::redundant_imports
            ),
            // A lint of another tool's namespace, or a path that does not name exactly one lint, is
            // not something we can reason about here, so assume the attribute is meaningful.
            _ => true,
        },
        ItemKind::ExternCrate(..) => {
            (skip_unused_imports && is_word(lint, sym::unused_imports)) || is_word(lint, sym::unused_extern_crates)
        },
        _ => false,
    }
}

pub(super) fn check(cx: &EarlyContext<'_>, item: &Item, attrs: &[Attribute]) {
    let skip_unused_imports = attrs.iter().any(|attr| attr.has_name(sym::macro_use));

    for attr in attrs {
        if let Some(lint_list) = &attr.meta_item_list()
            && attr.name().is_some_and(is_lint_level)
        {
            // Naming a single lint that is emitted on the item makes the whole attribute meaningful.
            // Note this only excuses the attribute in question; sibling attributes still have to be
            // checked on their own.
            let is_meaningful = lint_list
                .iter()
                .any(|lint| is_emitted_on_item(item, lint, skip_unused_imports));

            if !is_meaningful
                && !attr.span.in_external_macro(cx.sess().source_map())
                && let line_span = first_line_of_span(cx, attr.span)
                && let Some(src) = line_span.get_text(cx)
                && src.contains("#[")
            {
                #[expect(clippy::collapsible_span_lint_calls)]
                span_lint_and_then(cx, USELESS_ATTRIBUTE, line_span, "useless lint attribute", |diag| {
                    diag.span_suggestion(
                        line_span,
                        "if you just forgot a `!`, use",
                        src.replacen("#[", "#![", 1),
                        Applicability::MaybeIncorrect,
                    );
                });
            }
        }
    }
}

use super::USELESS_ATTRIBUTE;
use super::utils::{is_lint_level, is_word, namespace_and_lint};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{SpanExt as _, first_line_of_span};
use clippy_utils::sym;
use rustc_ast::{AttrStyle, Attribute, Item, ItemKind, MetaItemInner};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext as _};

/// Returns whether `lint` is emitted on a `use`/`extern crate` item itself. Such a lint makes a
/// lint level attribute naming it meaningful, so the attribute must not be reported.
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
            // A lint of another tool's namespace, or a path that does not name exactly one lint,
            // cannot be reasoned about here, so assume the attribute is meaningful.
            _ => true,
        },
        ItemKind::ExternCrate(..) => {
            (skip_unused_imports && is_word(lint, sym::unused_imports)) || is_word(lint, sym::unused_extern_crates)
        },
        _ => false,
    }
}

/// Returns whether `lint` is emitted on one of the item's own attributes rather than on the item
/// itself. `clippy::allow_attributes` only fires on an outer `allow` attribute, so permitting it
/// needs one to be there. Suppressing it is the documented workaround for an `allow` that is only
/// useless for some expansions of a macro, see
/// <https://github.com/rust-lang/rust-clippy/issues/17562>.
fn is_emitted_on_attrs(attrs: &[Attribute], lint: &MetaItemInner) -> bool {
    match namespace_and_lint(lint) {
        (Some(sym::clippy), Some(sym::allow_attributes)) => attrs
            .iter()
            .any(|attr| matches!(attr.style, AttrStyle::Outer) && attr.has_name(sym::allow)),
        _ => false,
    }
}

/// Returns whether a lint level attribute naming `lint_list` should be reported as useless, which
/// is the case when none of those lints is emitted on `item` itself or on its attributes.
fn is_useless(item: &Item, attrs: &[Attribute], lint_list: &[MetaItemInner], skip_unused_imports: bool) -> bool {
    !lint_list
        .iter()
        .any(|lint| is_emitted_on_item(item, lint, skip_unused_imports))
        && !lint_list.iter().any(|lint| is_emitted_on_attrs(attrs, lint))
}

pub(super) fn check(cx: &EarlyContext<'_>, item: &Item, attrs: &[Attribute]) {
    let skip_unused_imports = attrs.iter().any(|attr| attr.has_name(sym::macro_use));

    for attr in attrs {
        if let Some(lint_list) = &attr.meta_item_list()
            && attr.name().is_some_and(is_lint_level)
            && is_useless(item, attrs, lint_list, skip_unused_imports)
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

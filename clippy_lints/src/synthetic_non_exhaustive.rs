use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_doc_hidden;
use rustc_ast::ast::*;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Detects `enum`s that have hidden variants and are not marked as
    /// non-exhaustive.
    ///
    /// ### Why is this bad?
    /// `enum`s with hidden variants are effectively non-exhaustive, and should
    //  be marked as such to give clarity to consumers.
    ///
    /// ### Example
    /// ```rust
    /// pub enum Foo {
    ///     A,
    ///     B,
    ///     #[doc(hidden)]
    ///     C,
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[non_exhaustive]
    /// pub enum Foo {
    ///     A,
    ///     B,
    ///     #[doc(hidden)]
    ///     C,
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub SYNTHETIC_NON_EXHAUSTIVE,
    pedantic,
    "enum with hidden variants not marked as non-exhaustive"
}
declare_lint_pass!(SyntheticNonExhaustive => [SYNTHETIC_NON_EXHAUSTIVE]);

impl EarlyLintPass for SyntheticNonExhaustive {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if_chain! {
            if let ItemKind::Enum(ref def, _) = item.kind;
            if !item.attrs.iter().any(is_non_exhaustive);
            if def.variants.iter().any(is_hidden_variant);
            then {
                span_lint_and_help(
                    cx,
                    SYNTHETIC_NON_EXHAUSTIVE,
                    item.span,
                    "missing `#[non_exhaustive]` on an enum with hidden variants",
                    None,
                    "consider adding the `#[non_exhaustive]` attribute to the enum"
                );
            }
        }
    }
}

fn is_non_exhaustive(attr: &Attribute) -> bool {
    if_chain! {
        if let AttrKind::Normal(normal) = &attr.kind;
        if let [segment] = normal.item.path.segments.as_slice();
        then {
            segment.ident.name == sym::non_exhaustive
        } else {
            false
        }
    }
}

fn is_hidden_variant(variant: &Variant) -> bool {
    is_doc_hidden(&variant.attrs)
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks whether the traits in a `#[derive(...)]` list are in alphabetical/lexicographic order.
    ///
    /// ### Why is this bad?
    ///
    /// Having a consistent order makes the code more readable and maintainable.
    /// It also helps to avoid merge conflicts when multiple developers add traits
    /// to the same derive list.
    ///
    /// ### Example
    /// ```rust
    /// #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    /// struct Foo;
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    /// struct Foo;
    /// ```
    #[clippy::version = "1.85.0"]
    pub DERIVE_TRAIT_ORDERING,
    style,
    "traits in `#[derive(...)]` should be in alphabetical order"
}

declare_lint_pass!(DeriveTraitOrdering => [DERIVE_TRAIT_ORDERING]);

impl<'tcx> LateLintPass<'tcx> for DeriveTraitOrdering {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..) = item.kind {
            // Get all derive attributes on the item
            let attrs = cx.tcx.hir().attrs(item.hir_id());
            for attr in attrs {
                if attr.has_name(sym::derive) {
                    if let Some(list) = attr.meta_item_list() {
                        let mut traits: Vec<(String, rustc_span::Span)> = Vec::new();
                        
                        // Extract trait names and their spans
                        for meta_item in list {
                            if let Some(word) = meta_item.ident() {
                                // Skip items that are in derive expansions to avoid false positives
                                if !meta_item.span().in_derive_expansion() {
                                    traits.push((word.name.to_ident_string(), meta_item.span()));
                                }
                            }
                        }
                        
                        // Only check if we have more than one trait to sort
                        if traits.len() > 1 {
                            // Check if the traits are in alphabetical order
                            let original_order: Vec<&str> = traits.iter().map(|(name, _)| name.as_str()).collect();
                            let mut sorted_order = original_order.clone();
                            sorted_order.sort_unstable();
                            
                            if original_order != sorted_order {
                                // Create the fixed derive attribute - join with proper formatting
                                let fixed_derive = format!("#[derive({})]", sorted_order.join(", "));
                                
                                // Provide the lint with a suggestion
                                span_lint_and_sugg(
                                    cx,
                                    DERIVE_TRAIT_ORDERING,
                                    attr.span,
                                    "traits in `#[derive(...)]` are not in alphabetical order",
                                    "consider reordering the traits alphabetically",
                                    fixed_derive,
                                    Applicability::MachineApplicable,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
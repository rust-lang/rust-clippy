//! Records the spans of test code so that `allow-in-tests` covers early lint passes.
//!
//! A late pass resolves "is this test code?" by walking a node's HIR ancestors. Early passes have
//! no equivalent: `EarlyContext` carries only lint levels, not a parent chain. So instead of
//! asking the question per node, the answer is computed up front by walking the expanded AST once
//! and recording the span of every `#[cfg(test)]` item and `#[test]` function; emission then
//! matches a lint's span against those ranges.
//!
//! `check_crate` runs on the combined early pass before rustc visits any node, so the table is
//! always populated before the first lint can fire.

use clippy_utils::diagnostics::{any_lint_allowed_in_tests, set_test_code_spans};
use rustc_ast::attr::data_structures::CfgEntry;
use rustc_ast::visit::{self, Visitor};
use rustc_ast::{AttrKind, Attribute, Crate, Item, ItemKind, ModKind, SyntheticAttr};
use rustc_lint::{EarlyContext, EarlyLintPass, declare_lint_pass};
use rustc_span::{Span, Symbol, sym};

declare_lint_pass!(TestSpanCollector => []);

impl EarlyLintPass for TestSpanCollector {
    fn check_crate(&mut self, _: &EarlyContext<'_>, krate: &Crate) {
        // Nothing consults the spans unless the configuration names a lint, so don't pay for the
        // walk in the common case.
        if !any_lint_allowed_in_tests() {
            return;
        }
        let mut visitor = TestSpans { spans: Vec::new() };
        visitor.collect_test_fns(&krate.items);
        visit::walk_crate(&mut visitor, krate);
        set_test_code_spans(visitor.spans);
    }
}

struct TestSpans {
    spans: Vec<Span>,
}

impl TestSpans {
    /// Records the `#[test]` functions among `items`.
    ///
    /// Expansion consumes the `#[test]` attribute, leaving behind a `TestDescAndFn` constant
    /// named after the function and marked `#[rustc_test_marker]`. Matching the function to that
    /// constant by name is what `clippy_utils::is_in_test_function` does for late passes.
    fn collect_test_fns(&mut self, items: &[Box<Item>]) {
        let names: Vec<Symbol> = items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Const(konst) if item.attrs.iter().any(is_test_marker) => Some(konst.ident.name),
                _ => None,
            })
            .collect();
        if names.is_empty() {
            return;
        }
        for item in items {
            if let ItemKind::Fn(func) = &item.kind
                && names.contains(&func.ident.name)
            {
                self.spans.push(item.span);
            }
        }
    }
}

impl<'ast> Visitor<'ast> for TestSpans {
    fn visit_item(&mut self, item: &'ast Item) {
        if item.attrs.iter().any(is_cfg_test) {
            // Anything nested inside is covered by containment, so there's no need to descend.
            self.spans.push(item.span);
            return;
        }
        if let ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) = &item.kind {
            self.collect_test_fns(items);
        }
        visit::walk_item(self, item);
    }
}

/// Whether `attr` is the `#[cfg(test)]` left behind by expansion.
///
/// Only a top-level `test` is recognized, matching `clippy_utils::is_cfg_test`.
fn is_cfg_test(attr: &Attribute) -> bool {
    matches!(
        &attr.kind,
        AttrKind::Synthetic(synthetic)
            if matches!(&**synthetic, SyntheticAttr::CfgTrace(CfgEntry::NameValue { name: sym::test, .. }))
    )
}

/// Whether `attr` is the `#[rustc_test_marker]` expansion puts on a test's descriptor constant.
fn is_test_marker(attr: &Attribute) -> bool {
    matches!(attr.kind, AttrKind::Normal(_)) && attr.has_name(sym::rustc_test_marker)
}

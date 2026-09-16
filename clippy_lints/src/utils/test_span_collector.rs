//! Records the spans of test code so that `allow-in-tests` can cover early lint passes.
//!
//! This method is needed because only late pass lints have access to
//! the HIR, which contains more accurate parent node attribute information.

use clippy_utils::diagnostics::{is_a_lint_allowed_in_tests, set_test_code_spans};
use rustc_ast::attr::data_structures::CfgEntry;
use rustc_ast::visit::{self, Visitor};
use rustc_ast::{AttrKind, Attribute, Crate, Item, ItemKind, ModKind, SyntheticAttr};
use rustc_lint::{EarlyContext, EarlyLintPass, declare_lint_pass};
use rustc_span::{Span, Symbol, sym};

declare_lint_pass!(TestSpanCollector => []);

impl EarlyLintPass for TestSpanCollector {
    fn check_crate(&mut self, _: &EarlyContext<'_>, krate: &Crate) {
        if !is_a_lint_allowed_in_tests() {
            // The cache of test spans will never be consulted, so don't
            // bother calculating test spans.
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

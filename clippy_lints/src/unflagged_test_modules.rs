use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{symbol::sym, BytePos};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Triggers when a testing module (that contains "test" or "tests" in its name) isn't flagged with the `#[cfg(test)]` attribute.
    ///
    /// ### Why is this bad?
    ///
    /// The attribute `#[cfg(test)]` is used to tell Rust to compile and run the test code only when you run `cargo test` and not when you run `cargo  build`. This saves compile time and space in the resulting compiled artifact because tests are not included. So not using `#[cfg(test)]` for tests is both a waste of time and space.
    ///
    /// ### Example
    /// ```rust
    /// mod my_cool_tests {
    /// 	// [...]
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[cfg(test)]
    /// mod my_cool_tests {
    /// 	// [...]
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub UNFLAGGED_TEST_MODULES,
    pedantic,
    "the testing module `my_cool_tests` wasn't marked with `#[cfg(test)]`"
}
declare_lint_pass!(UnflaggedTestModules => [UNFLAGGED_TEST_MODULES]);

impl LateLintPass<'_> for UnflaggedTestModules {
    fn check_item_post(&mut self, cx: &LateContext<'_>, item: &rustc_hir::Item<'_>) {
        if let ItemKind::Mod(_) = item.kind {
            // If module name contains *test* or *tests*.
            if item
                .ident
                .name
                .to_ident_string()
                .split('_')
                .any(|seg| seg == "test" || seg == "tests")
            {
                for attr in cx.tcx.get_attrs(item.owner_id.to_def_id(), sym::cfg) {
                    if_chain! {
                        if attr.has_name(sym::cfg);
                        if let Some(items) = attr.meta_item_list();
                        if let [item] = &*items;
                        if item.has_name(sym::test);
                        then {
                            return;
                        }
                    }
                }
                // If no #[cfg(test)] is found
                span_lint_and_sugg(
                    cx,
                    UNFLAGGED_TEST_MODULES,
                    item.ident.span.with_lo(
                        item.ident.span.lo() - BytePos(4), // Add `mod` keyword
                    ),
                    "this testing module isn't flagged with #[cfg(test)]",
                    "add the attribute",
                    format!("#[cfg(test)]\nmod {}", item.ident.as_str()),
                    rustc_errors::Applicability::MachineApplicable,
                );
            }
        }
    }
}

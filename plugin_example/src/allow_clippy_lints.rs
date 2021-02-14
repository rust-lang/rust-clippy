use clippy_utils::{declare_clippy_lint, span_lint_and_sugg};
use if_chain::if_chain;
use rustc_ast::{Attribute, NestedMetaItem};
use rustc_errors::Applicability;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;
use rustc_span::symbol::SymbolStr;

declare_clippy_lint! {
    /// **What it does:** This tongue-in-cheek lint checks for `#[allow(clippy::...)]`.
    /// It is based on `blanket_clippy_restriction_lints`:
    /// https://rust-lang.github.io/rust-clippy/master/#blanket_clippy_restriction_lints
    ///
    /// **Why is this bad?** It's not really. This is just an example of a Clippy plugin.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// Bad:
    /// ```rust
    /// #![allow(clippy::assertions_on_constants)]
    /// ```
    ///
    /// Good:
    /// ```rust
    /// #[deny(clippy::restriction, clippy::style, clippy::pedantic, clippy::complexity, clippy::perf, clippy::cargo, clippy::nursery)]
    /// ```
    pub ALLOW_CLIPPY_LINTS,
    correctness,
    "use of `#[allow(clippy::...)]`"
}

declare_lint_pass!(AllowClippyLints => [
    ALLOW_CLIPPY_LINTS,
]);

impl<'tcx> LateLintPass<'tcx> for AllowClippyLints {
    fn check_attribute(&mut self, cx: &LateContext<'tcx>, attr: &'tcx Attribute) {
        if let Some(items) = &attr.meta_item_list() {
            if let Some(ident) = attr.ident() {
                let ident = &*ident.as_str();
                if ident == "allow" {
                    check_clippy_lint_names(cx, &attr, items);
                }
            }
        }
    }
}

/// Returns the lint name if it is clippy lint.
fn extract_clippy_lint(lint: &NestedMetaItem) -> Option<SymbolStr> {
    if_chain! {
        if let Some(meta_item) = lint.meta_item();
        if meta_item.path.segments.len() > 1;
        if let tool_name = meta_item.path.segments[0].ident;
        if tool_name.name == sym::clippy;
        let lint_name = meta_item.path.segments.last().unwrap().ident.name;
        then {
            return Some(lint_name.as_str());
        }
    }
    None
}

fn check_clippy_lint_names(cx: &LateContext<'_>, attr: &Attribute, items: &[NestedMetaItem]) {
    if items.iter().find_map(extract_clippy_lint).is_some() {
        span_lint_and_sugg(
            cx,
            ALLOW_CLIPPY_LINTS,
            attr.span,
            "`allow`ing Clippy lints `deny`s your project of its true potential",
            "use",
            "#[deny(clippy::restriction, clippy::style, clippy::pedantic, clippy::complexity, clippy::perf, clippy::cargo, clippy::nursery)]".to_string(),
            Applicability::MachineApplicable,
        );
    }
}

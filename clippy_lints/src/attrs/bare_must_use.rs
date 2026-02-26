//! Lint to enforce adding a reason to `#[must_use]` attributes

use super::BAREMUSTUSE;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::Attribute;
use rustc_lint::LintContext;
use rustc_span::sym;

pub(super) fn check(cx: &rustc_lint::EarlyContext<'_>, attr: &Attribute) {
    // Check if this is a must_use attribute
    if !attr.has_name(sym::must_use) {
        return;
    }

    // Check if it's in an external macro
    if attr.span.in_external_macro(cx.sess().source_map()) {
        return;
    }

    // Check if there's a reason (the optional argument to must_use)
    if attr.meta_item_list().is_some() {
        // has arguments like #[must_use = "reason"]
        return;
    }

    // No reason specified - emit the lint
    span_lint_and_help(
        cx,
        BAREMUSTUSE,
        attr.span,
        "#[must_use] attribute without a reason",
        None,
        "try adding a reason, e.g., `#[must_use = \"computing this is expensive\"]`",
    );
}

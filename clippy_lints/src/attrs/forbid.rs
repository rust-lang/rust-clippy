use super::utils::extract_clippy_lint;
use super::{FORBID, HYPOCRISY};
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use rustc_ast::Attribute;
use rustc_lint::EarlyContext;
use rustc_span::sym;

pub(super) fn check(cx: &EarlyContext<'_>, attr: &Attribute) {
    if !attr.has_name(sym::forbid) {
        return;
    }

    let Some(lints) = attr.meta_item_list() else {
        return;
    };

    if lints.is_empty() {
        return;
    }

    let is_hypocritical = lints
        .iter()
        .any(|lint| extract_clippy_lint(lint).is_some_and(|l| l == sym::forbid));

    if is_hypocritical {
        span_lint(
            cx,
            HYPOCRISY,
            attr.span,
            "you have declared `#[forbid(clippy::forbid)]`, which is hypocritical",
        );
    } else {
        span_lint_and_help(
            cx,
            FORBID,
            attr.span,
            "you have used the `forbid` attribute",
            None,
            "consider using `deny` instead",
        );
    }
}

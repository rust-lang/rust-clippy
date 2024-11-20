use clippy_utils::diagnostics::span_lint_and_then;
use itertools::Itertools;
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_span::Span;
use std::ops::Range;

use super::DOC_OVERINDENTED_LIST_ITEMS;

pub(super) fn check(cx: &LateContext<'_>, doc: &str, range: Range<usize>, span: Span, containers: &[super::Container]) {
    if doc[range.clone()].contains('\t') {
        // We don't do tab stops correctly.
        return;
    }

    let is_blockquote = containers.iter().any(|c| matches!(c, super::Container::Blockquote));
    if is_blockquote {
        // If this doc is a blockquote, we don't go further.
        return;
    }

    let leading_spaces = doc[range].chars().filter(|c| *c == ' ').count();
    let list_indentation = containers
        .iter()
        .map(|c| {
            if let super::Container::List(indent) = c {
                *indent
            } else {
                0
            }
        })
        .sum();

    if leading_spaces > list_indentation {
        span_lint_and_then(
            cx,
            DOC_OVERINDENTED_LIST_ITEMS,
            span,
            "doc list item overindented",
            |diag| {
                diag.span_suggestion_verbose(
                    span,
                    "remove unnecessary spaces",
                    std::iter::repeat(" ").take(list_indentation).join(""),
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

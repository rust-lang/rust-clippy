use clippy_utils::diagnostics::span_lint_and_then;
use itertools::Itertools;
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_span::{BytePos, Span};
use std::ops::Range;

use super::DOC_LAZY_CONTINUATION;

fn map_container_to_text(c: &super::Container) -> &'static str {
    match c {
        super::Container::Blockquote => "> ",
        // numbered list can have up to nine digits, plus the dot, plus four spaces on either side
        super::Container::List(indent) => &"                  "[0..*indent],
    }
}

// TODO: Adjust the parameters as necessary
pub(super) fn check(
    cx: &LateContext<'_>,
    doc: &str,
    range: Range<usize>,
    mut span: Span,
    containers: &[super::Container],
) {
    if doc[range.clone()].contains('\t') {
        // We don't do tab stops correctly.
        return;
    }

    // Handle blockquotes.
    let ccount = doc[range.clone()].chars().filter(|c| *c == '>').count();
    let blockquote_level = containers
        .iter()
        .filter(|c| matches!(c, super::Container::Blockquote))
        .count();
    if ccount < blockquote_level {
        let msg = "doc quote line without `>` marker";
        span_lint_and_then(cx, DOC_LAZY_CONTINUATION, span, msg, |diag| {
            let mut doc_start_range = &doc[range];
            let mut suggested = String::new();
            for c in containers {
                let text = map_container_to_text(c);
                if doc_start_range.starts_with(text) {
                    doc_start_range = &doc_start_range[text.len()..];
                    span = span
                        .with_lo(span.lo() + BytePos(u32::try_from(text.len()).expect("text is not 2**32 or bigger")));
                } else if matches!(c, super::Container::Blockquote)
                    && let Some(i) = doc_start_range.find('>')
                {
                    doc_start_range = &doc_start_range[i + 1..];
                    span =
                        span.with_lo(span.lo() + BytePos(u32::try_from(i).expect("text is not 2**32 or bigger") + 1));
                } else {
                    suggested.push_str(text);
                }
            }
            diag.span_suggestion_verbose(
                span,
                "add markers to start of line",
                suggested,
                Applicability::MachineApplicable,
            );
            diag.help("if this not intended to be a quote at all, escape it with `\\>`");
        });
        return;
    }

    if ccount != 0 || blockquote_level != 0 {
        // If this doc is a blockquote, we don't go further.
        return;
    }

    // Handle list items
    let lcount = doc[range.clone()].chars().filter(|c| *c == ' ').count();
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
    if lcount != list_indentation {
        let msg = if lcount < list_indentation {
            "doc list item without indentation"
        } else {
            "doc list item overindented"
        };
        span_lint_and_then(cx, DOC_LAZY_CONTINUATION, span, msg, |diag| {
            if lcount < list_indentation {
                // simpler suggestion style for indentation
                let indent = list_indentation - lcount;
                diag.span_suggestion_verbose(
                    span.shrink_to_hi(),
                    "indent this line",
                    std::iter::repeat(" ").take(indent).join(""),
                    Applicability::MaybeIncorrect,
                );
            } else {
                diag.span_suggestion_verbose(
                    span,
                    "indent this line",
                    std::iter::repeat(" ").take(list_indentation).join(""),
                    Applicability::MaybeIncorrect,
                );
            }
            diag.help("if this is supposed to be its own paragraph, add a blank line");
        });
    }
}

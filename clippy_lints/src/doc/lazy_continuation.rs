use clippy_utils::diagnostics::span_lint_and_then;
use itertools::Itertools;
use rustc_errors::{Applicability, SuggestionStyle};
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
    line_break_span: Span,
) {
    if doc[range.clone()].contains('\t') {
        // We don't do tab stops correctly.
        return;
    }

    let ccount = doc[range.clone()].chars().filter(|c| *c == '>').count();
    let blockquote_level = containers
        .iter()
        .filter(|c| matches!(c, super::Container::Blockquote))
        .count();
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
    let list_indentation_less_strict = if list_indentation > 2 && blockquote_level == 0 {
        // This is technically still a lazy continuation, but it's not very confusing.
        // To make sure it's not very confusing, we also have to be careful of block quote markers,
        // because they'll eat a space afterward.
        list_indentation - 1
    } else {
        list_indentation
    };
    if ccount < blockquote_level || lcount < list_indentation_less_strict {
        let msg = if ccount < blockquote_level {
            "doc quote line without `>` marker"
        } else {
            "doc list item without indentation"
        };
        span_lint_and_then(cx, DOC_LAZY_CONTINUATION, span, msg, |diag| {
            let snippet = clippy_utils::source::snippet(cx, line_break_span, "");
            if snippet.chars().filter(|&c| c == '\n').count() > 1
                && let Some(doc_comment_start) = snippet.rfind('\n')
                && let doc_comment = snippet[doc_comment_start..].trim()
                && (doc_comment == "///" || doc_comment == "//!")
            {
                // suggest filling in a blank line
                diag.span_suggestion_with_style(
                    line_break_span.shrink_to_lo(),
                    "if this should be its own paragraph, add a blank doc comment line",
                    format!("\n{doc_comment}"),
                    Applicability::MaybeIncorrect,
                    SuggestionStyle::ShowAlways,
                );
                if ccount > 0 || blockquote_level > 0 {
                    diag.help("if this not intended to be a quote at all, escape it with `\\>`");
                } else {
                    let indent = list_indentation - lcount;
                    diag.help(format!(
                        "if this is intended to be part of the list, indent {indent} spaces"
                    ));
                }
                return;
            }
            if ccount == 0 && blockquote_level == 0 {
                // simpler suggestion style for indentation
                let indent = list_indentation - lcount;
                diag.span_suggestion_with_style(
                    span.shrink_to_hi(),
                    "indent this line",
                    std::iter::repeat(" ").take(indent).join(""),
                    Applicability::MaybeIncorrect,
                    SuggestionStyle::ShowAlways,
                );
                diag.help("if this is supposed to be its own paragraph, add a blank line");
                return;
            }
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
            diag.span_suggestion_with_style(
                span,
                "add markers to start of line",
                suggested,
                Applicability::MachineApplicable,
                SuggestionStyle::ShowAlways,
            );
            diag.help("if this not intended to be a quote at all, escape it with `\\>`");
        });
    }
}

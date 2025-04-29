use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_lint::LateContext;

use std::ops::Range;

use super::{DOC_SUSPICIOUS_FOOTNOTES, Fragments};

pub fn check(cx: &LateContext<'_>, doc: &str, range: Range<usize>, fragments: &Fragments<'_>) {
    for i in doc[range.clone()]
        .bytes()
        .enumerate()
        .filter_map(|(i, c)| if c == b'[' { Some(i) } else { None })
    {
        let start = i + range.start;
        if doc.as_bytes().get(start + 1) == Some(&b'^')
            && let Some(end) = all_numbers_upto_brace(doc, start + 2)
            && doc.as_bytes().get(end) != Some(&b':')
            && doc.as_bytes().get(start - 1) != Some(&b'\\')
            && let Some(span) = fragments.span(cx, start..end)
        {
            span_lint_and_then(
                cx,
                DOC_SUSPICIOUS_FOOTNOTES,
                span,
                "looks like a footnote ref, but no matching footnote",
                |diag| {
                    let mut applicability = Applicability::MachineApplicable;
                    let snippet = snippet_with_applicability(cx, span, "..", &mut applicability);
                    diag.span_suggestion_verbose(span, "try", format!("`{snippet}`"), applicability);
                },
            );
        }
    }
}

fn all_numbers_upto_brace(text: &str, i: usize) -> Option<usize> {
    for (j, c) in text.as_bytes()[i..].iter().copied().enumerate().take(64) {
        if c == b']' && j != 0 {
            return Some(i + j + 1);
        }
        if !c.is_ascii_digit() || j >= 64 {
            break;
        }
    }
    None
}

use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_lint::{LateContext, LintContext};

use std::ops::Range;

use super::{DOC_SUSPICIOUS_FOOTNOTES, Fragments};

pub fn check(cx: &LateContext<'_>, doc: &str, range: Range<usize>, fragments: &Fragments<'_>) {
    for i in doc[range.clone()]
        .bytes()
        .enumerate()
        .filter_map(|(i, c)| if c == b'[' { Some(i) } else { None })
    {
        let start = i + range.start;
        let mut this_fragment_start = start;
        if doc.as_bytes().get(start + 1) == Some(&b'^')
            && let Some(end) = all_numbers_upto_brace(doc, start + 2)
            && doc.as_bytes().get(end) != Some(&b':')
            && doc.as_bytes().get(start - 1) != Some(&b'\\')
            && let Some(this_fragment) = fragments
                .fragments
                .iter()
                .find(|frag| {
                    let found = this_fragment_start < frag.doc.as_str().len();
                    if !found {
                        this_fragment_start -= frag.doc.as_str().len();
                    }
                    found
                })
                .or(fragments.fragments.last())
        {
            let span = fragments.span(cx, start..end).unwrap_or(this_fragment.span);
            span_lint_and_then(
                cx,
                DOC_SUSPICIOUS_FOOTNOTES,
                span,
                "looks like a footnote ref, but has no matching footnote",
                |diag| {
                    let applicability = Applicability::HasPlaceholders;
                    let start_of_md_line = doc.as_bytes()[..start]
                        .iter()
                        .rposition(|&c| c == b'\n' || c == b'\r')
                        .unwrap_or(0);
                    let end_of_md_line = doc.as_bytes()[start..]
                        .iter()
                        .position(|&c| c == b'\n' || c == b'\r')
                        .unwrap_or(doc.len() - start)
                        + start;
                    let span_md_line = fragments
                        .span(cx, start_of_md_line..end_of_md_line)
                        .unwrap_or(this_fragment.span);
                    let span_whole_line = cx.sess().source_map().span_extend_to_line(span_md_line);
                    if let Ok(mut pfx) = cx
                        .sess()
                        .source_map()
                        .span_to_snippet(span_whole_line.until(span_md_line))
                        && let Ok(mut sfx) = cx
                            .sess()
                            .source_map()
                            .span_to_snippet(span_md_line.shrink_to_hi().until(span_whole_line.shrink_to_hi()))
                    {
                        let mut insert_before = String::new();
                        let mut insert_after = String::new();
                        let span = if this_fragment.kind == rustc_resolve::rustdoc::DocFragmentKind::RawDoc
                            && (!pfx.is_empty() || !sfx.is_empty())
                        {
                            if (pfx.trim() == "#[doc=" || pfx.trim() == "#![doc=") && sfx.trim() == "]" {
                                // try to use per-line doc fragments if that's what the author did
                                pfx.push('"');
                                sfx.insert(0, '"');
                                span_whole_line.shrink_to_hi()
                            } else {
                                // otherwise, replace the whole line with the result
                                pfx = String::new();
                                sfx = String::new();
                                insert_before = format!(r#"r###"{}"#, this_fragment.doc);
                                r####""###"####.clone_into(&mut insert_after);
                                span_md_line
                            }
                        } else {
                            span_whole_line.shrink_to_hi()
                        };
                        diag.span_suggestion_verbose(
                            span,
                            "add footnote definition",
                            format!("{insert_before}\n{pfx}{sfx}\n{pfx}{label}: <!-- description -->{sfx}\n{pfx}{sfx}{insert_after}", label = &doc[start..end]),
                            applicability,
                        );
                    }
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

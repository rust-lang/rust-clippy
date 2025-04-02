use clippy_utils::diagnostics::span_lint;
use pulldown_cmark::BrokenLink as PullDownBrokenLink;
use rustc_lint::LateContext;
use rustc_resolve::rustdoc::{DocFragment, source_span_for_markdown_range};
use rustc_span::{BytePos, Pos, Span};

use super::DOC_BROKEN_LINK;

/// Scan and report broken link on documents.
/// It ignores false positives detected by pulldown_cmark, and only
/// warns users when the broken link is consider a URL.
pub fn check(cx: &LateContext<'_>, bl: &PullDownBrokenLink<'_>, doc: &String, fragments: &Vec<DocFragment>) {
    warn_if_broken_link(cx, bl, doc, fragments);
}

/// The reason why a link is considered broken.
// NOTE: We don't check these other cases because
// rustdoc itself will check and warn about it:
// - When a link url is broken across multiple lines in the URL path part
// - When a link tag is missing the close parenthesis character at the end.
// - When a link has whitespace within the url link.
enum BrokenLinkReason {
    MultipleLines,
}

fn warn_if_broken_link(cx: &LateContext<'_>, bl: &PullDownBrokenLink<'_>, doc: &String, fragments: &Vec<DocFragment>) {
    if let Some(span) = source_span_for_markdown_range(cx.tcx, doc, &bl.span, fragments) {
        let mut len = 0;

        // grab raw link data
        let (_, raw_link) = doc.split_at(bl.span.start);

        // strip off link text part
        let raw_link = match raw_link.split_once(']') {
            None => return,
            Some((prefix, suffix)) => {
                len += prefix.len() + 1;
                suffix
            },
        };

        let raw_link = match raw_link.split_once('(') {
            None => return,
            Some((prefix, suffix)) => {
                if !prefix.is_empty() {
                    // there is text between ']' and '(' chars, so it is not a valid link
                    return;
                }
                len += prefix.len() + 1;
                suffix
            },
        };

        for c in raw_link.chars() {
            if c == ')' {
                // it is a valid link
                return;
            }

            if c == '\n' {
                // detected break line within the url part
                report_broken_link(cx, span, len, BrokenLinkReason::MultipleLines);
                break;
            }
            len += 1;
        }
    }
}

fn report_broken_link(cx: &LateContext<'_>, frag_span: Span, offset: usize, reason: BrokenLinkReason) {
    let start = frag_span.lo();
    let end = start + BytePos::from_usize(offset);

    let span = Span::new(start, end, frag_span.ctxt(), frag_span.parent());

    let reason_msg = match reason {
        BrokenLinkReason::MultipleLines => "broken across multiple lines",
    };

    span_lint(
        cx,
        DOC_BROKEN_LINK,
        span,
        format!("possible broken doc link: {reason_msg}"),
    );
}

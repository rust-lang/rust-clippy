use std::mem;
use std::ops::Range;

use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_resolve::rustdoc::pulldown_cmark::{Event, Tag, TagEnd};

use crate::doc::Fragments;

use super::DOC_LINK_CODE;

struct PendingLink {
    range: Range<usize>,
    seen_code: bool,
}

#[derive(Default)]
pub(super) struct LinkCode {
    start: Option<usize>,
    end: Option<usize>,
    includes_link: bool,
    pending_link: Option<PendingLink>,
}

impl LinkCode {
    pub fn check(
        &mut self,
        cx: &LateContext<'_>,
        event: &Event<'_>,
        range: Range<usize>,
        doc: &str,
        fragments: Fragments<'_>,
    ) {
        match event {
            Event::Start(Tag::Link { .. }) => {
                self.pending_link = Some(PendingLink {
                    range,
                    seen_code: false,
                });
            },
            Event::End(TagEnd::Link) => {
                if let Some(PendingLink { range, seen_code: true }) = self.pending_link.take() {
                    if self.start.is_some() {
                        self.end = Some(range.end);
                    } else {
                        self.start = Some(range.start);
                    }
                    self.includes_link = true;
                }
            },
            _ if let Some(pending_link) = &mut self.pending_link => {
                if matches!(event, Event::Code(_)) && !pending_link.seen_code {
                    pending_link.seen_code = true;
                } else {
                    self.consume(cx, fragments, doc);
                }
            },
            Event::Code(_) => {
                if self.start.is_some() {
                    self.end = Some(range.end);
                } else {
                    self.start = Some(range.start);
                }
            },
            _ => self.consume(cx, fragments, doc),
        }
    }

    fn consume(&mut self, cx: &LateContext<'_>, fragments: Fragments<'_>, doc: &str) {
        if let LinkCode {
            start: Some(start),
            end: Some(end),
            includes_link: true,
            pending_link: _,
        } = mem::take(self)
            && let Some(span) = fragments.span(cx, start..end)
        {
            span_lint_and_then(cx, DOC_LINK_CODE, span, "code link adjacent to code text", |diag| {
                diag.span_suggestion_verbose(
                    span,
                    "wrap the entire group in `<code>` tags",
                    format!("<code>{}</code>", doc[start..end].replace('`', "")),
                    Applicability::MaybeIncorrect,
                );
                diag.help("separate code snippets will be shown with a gap");
            });
        }
    }
}

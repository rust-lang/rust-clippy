use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_resolve::rustdoc::pulldown_cmark::{Event, Tag, TagEnd};
use rustc_span::Span;
use std::ops::Range;

use super::{DOC_PARAGRAPHS_MISSING_PUNCTUATION, Fragments};

#[derive(Default)]
pub(super) struct MissingPunctuation {
    no_report_depth: u32,
    current_paragraph: Option<Position>,
}

impl MissingPunctuation {
    pub fn check(
        &mut self,
        cx: &LateContext<'_>,
        event: &Event<'_>,
        range: Range<usize>,
        doc: &str,
        fragments: Fragments<'_>,
    ) {
        // The colon is not exactly a terminal punctuation mark, but this is required for paragraphs that
        // introduce a table or a list for example.
        const TERMINAL_PUNCTUATION_MARKS: &[char] = &['.', '?', '!', '…', ':'];

        match event {
            Event::Start(
                Tag::CodeBlock(..)
                | Tag::FootnoteDefinition(_)
                | Tag::Heading { .. }
                | Tag::HtmlBlock
                | Tag::List(..)
                | Tag::Table(_),
            ) => {
                self.no_report_depth += 1;
            },
            Event::End(TagEnd::FootnoteDefinition) => {
                self.no_report_depth -= 1;
            },
            Event::End(
                TagEnd::CodeBlock | TagEnd::Heading(_) | TagEnd::HtmlBlock | TagEnd::List(_) | TagEnd::Table,
            ) => {
                self.no_report_depth -= 1;
                self.current_paragraph = None;
            },
            Event::InlineHtml(_) | Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {
                self.current_paragraph = None;
            },
            Event::End(TagEnd::Paragraph) => {
                if let Some(position) = self.current_paragraph
                    && let Some(span) = position.span(cx, fragments)
                {
                    span_lint_and_then(
                        cx,
                        DOC_PARAGRAPHS_MISSING_PUNCTUATION,
                        span,
                        "doc paragraphs should end with a terminal punctuation mark",
                        |diag| {
                            if matches!(position, Position::Fixable(_)) {
                                diag.span_suggestion(
                                    span,
                                    "end the paragraph with some punctuation",
                                    '.',
                                    Applicability::MaybeIncorrect,
                                );
                            } else {
                                diag.help("end the paragraph with some punctuation");
                            }
                        },
                    );
                }
            },
            Event::Code(..) | Event::Start(Tag::Link { .. }) | Event::End(TagEnd::Link)
                if self.no_report_depth == 0 && !range.is_empty() =>
            {
                if doc[..range.end].trim_end().ends_with(TERMINAL_PUNCTUATION_MARKS) {
                    self.current_paragraph = None;
                } else {
                    self.current_paragraph = Some(Position::Fixable(range.end));
                }
            },
            Event::Text(..) if self.no_report_depth == 0 && !range.is_empty() => {
                let trimmed = doc[..range.end].trim_end();
                if trimmed.ends_with(TERMINAL_PUNCTUATION_MARKS) {
                    self.current_paragraph = None;
                } else if let Some(t) = trimmed.strip_suffix(|c| c == ')' || c == '"') {
                    if t.ends_with(TERMINAL_PUNCTUATION_MARKS) {
                        // Avoid false positives.
                        self.current_paragraph = None;
                    } else {
                        self.current_paragraph = Some(Position::Unfixable(range.end));
                    }
                } else {
                    self.current_paragraph = Some(Position::Fixable(range.end));
                }
            },
            _ => {},
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Position {
    Fixable(usize),
    Unfixable(usize),
}

impl Position {
    fn span(self, cx: &LateContext<'_>, fragments: Fragments<'_>) -> Option<Span> {
        let (Position::Fixable(pos) | Position::Unfixable(pos)) = self;
        fragments.span(cx, pos..pos)
    }
}

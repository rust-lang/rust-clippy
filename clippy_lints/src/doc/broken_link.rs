use clippy_utils::diagnostics::span_lint;
use rustc_ast::{AttrKind, AttrStyle, Attribute};
use rustc_lint::LateContext;
use rustc_span::{BytePos, Span};

use super::DOC_BROKEN_LINK;

pub fn check(cx: &LateContext<'_>, attrs: &[Attribute]) {
    for broken_link in BrokenLinkLoader::collect_broken_links(attrs) {
        let reason_msg = match broken_link.reason {
            BrokenLinkReason::MultipleLines => "broken across multiple lines",
        };

        span_lint(
            cx,
            DOC_BROKEN_LINK,
            broken_link.span,
            format!("possible broken doc link: {reason_msg}"),
        );
    }
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

/// Broken link data.
struct BrokenLink {
    reason: BrokenLinkReason,
    span: Span,
}

enum State {
    ProcessingLinkText,
    ProcessedLinkText,
    ProcessingLinkUrl(UrlState),
}

enum UrlState {
    Empty,
    FilledEntireSingleLine,
    FilledBrokenMultipleLines,
}

/// Scan AST attributes looking up in doc comments for broken links
/// which rustdoc won't be able to properly create link tags later.
struct BrokenLinkLoader {
    /// List of detected broken links.
    broken_links: Vec<BrokenLink>,

    state: Option<State>,

    /// Keep track of the span for the processing broken link.
    active_span: Option<Span>,

    /// Keep track where exactly the link definition has started in the code.
    active_pos_start: u32,
}

impl BrokenLinkLoader {
    /// Return broken links.
    fn collect_broken_links(attrs: &[Attribute]) -> Vec<BrokenLink> {
        let mut loader = BrokenLinkLoader {
            broken_links: vec![],
            state: None,
            active_pos_start: 0,
            active_span: None,
        };
        loader.scan_attrs(attrs);
        loader.broken_links
    }

    fn scan_attrs(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            if let AttrKind::DocComment(_com_kind, sym) = attr.kind
                && let AttrStyle::Outer = attr.style
            {
                self.scan_line(sym.as_str(), attr.span);
            }
        }
    }

    fn scan_line(&mut self, line: &str, attr_span: Span) {
        // Note that we specifically need the char _byte_ indices here, not the positional indexes
        // within the char array to deal with multi-byte characters properly. `char_indices` does
        // exactly that. It provides an iterator over tuples of the form `(byte position, char)`.
        let char_indices: Vec<_> = line.char_indices().collect();

        let reading_link_url_new_line = matches!(
            self.state,
            Some(State::ProcessingLinkUrl(UrlState::FilledEntireSingleLine))
        );

        for (pos, c) in char_indices {
            if pos == 0 && c.is_whitespace() {
                // ignore prefix whitespace on comments
                continue;
            }

            match &self.state {
                None => {
                    if c == '[' {
                        self.state = Some(State::ProcessingLinkText);
                        // +3 skips the opening delimiter
                        self.active_pos_start = attr_span.lo().0 + u32::try_from(pos).unwrap() + 3;
                        self.active_span = Some(attr_span);
                    }
                },
                Some(State::ProcessingLinkText) => {
                    if c == ']' {
                        self.state = Some(State::ProcessedLinkText);
                    }
                },
                Some(State::ProcessedLinkText) => {
                    if c == '(' {
                        self.state = Some(State::ProcessingLinkUrl(UrlState::Empty));
                    } else {
                        // not a real link, start lookup over again
                        self.reset_lookup();
                    }
                },
                Some(State::ProcessingLinkUrl(url_state)) => {
                    if c == ')' {
                        // record full broken link tag
                        if let UrlState::FilledBrokenMultipleLines = url_state {
                            // +3 skips the opening delimiter and +1 to include the closing parethesis
                            let pos_end = attr_span.lo().0 + u32::try_from(pos).unwrap() + 4;
                            self.record_broken_link(pos_end, BrokenLinkReason::MultipleLines);
                            self.reset_lookup();
                        }
                        self.reset_lookup();
                        continue;
                    }

                    if !c.is_whitespace() {
                        if reading_link_url_new_line {
                            // It was reading a link url which was entirely in a single line, but a new char
                            // was found in this new line which turned the url into a broken state.
                            self.state = Some(State::ProcessingLinkUrl(UrlState::FilledBrokenMultipleLines));
                            continue;
                        }

                        self.state = Some(State::ProcessingLinkUrl(UrlState::FilledEntireSingleLine));
                    }
                },
            };
        }
    }

    fn reset_lookup(&mut self) {
        self.state = None;
        self.active_span = None;
        self.active_pos_start = 0;
    }

    fn record_broken_link(&mut self, pos_end: u32, reason: BrokenLinkReason) {
        if let Some(attr_span) = self.active_span {
            let start = BytePos(self.active_pos_start);
            let end = BytePos(pos_end);

            let span = Span::new(start, end, attr_span.ctxt(), attr_span.parent());

            self.broken_links.push(BrokenLink { reason, span });
        }
    }
}

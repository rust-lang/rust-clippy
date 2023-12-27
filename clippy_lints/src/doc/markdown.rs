use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::source::snippet_with_applicability;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::{Applicability, SuggestionStyle};
use rustc_lint::LateContext;
use rustc_span::{BytePos, Pos, Span};
use url::Url;

use crate::doc::DOC_MARKDOWN;

pub fn check(cx: &LateContext<'_>, valid_idents: &FxHashSet<String>, text: &str, span: Span) {
    for orig_word in text.split(|c: char| c.is_whitespace() || c == '\'') {
        // Trim punctuation as in `some comment (see foo::bar).`
        //                                                   ^^
        // Or even as in `_foo bar_` which is emphasized. Also preserve `::` as a prefix/suffix.
        let trim_pattern = |c: char| !c.is_alphanumeric() && c != ':';
        let mut word = orig_word.trim_end_matches(trim_pattern);

        // If word is immediately followed by `()`, claw it back.
        if let Some(tmp_word) = orig_word.get(..word.len() + 2)
            && tmp_word.ends_with("()")
        {
            word = tmp_word;
        }

        word = word.trim_start_matches(trim_pattern);

        // Remove leading or trailing single `:` which may be part of a sentence.
        if word.starts_with(':') && !word.starts_with("::") {
            word = word.trim_start_matches(':');
        }
        if word.ends_with(':') && !word.ends_with("::") {
            word = word.trim_end_matches(':');
        }

        if valid_idents.contains(word) || word.chars().all(|c| c == ':') {
            continue;
        }

        // Adjust for the current word
        let offset = word.as_ptr() as usize - text.as_ptr() as usize;
        let span = Span::new(
            span.lo() + BytePos::from_usize(offset),
            span.lo() + BytePos::from_usize(offset + word.len()),
            span.ctxt(),
            span.parent(),
        );

        check_word(cx, word, span);
    }
}

fn check_word(cx: &LateContext<'_>, word: &str, span: Span) {
    /// Checks if a string is upper-camel-case, i.e., starts with an uppercase and
    /// contains at least two uppercase letters (`Clippy` is ok) and one lower-case
    /// letter (`NASA` is ok).
    /// Plurals are also excluded (`IDs` is ok).
    fn is_camel_case(s: &str) -> bool {
        if s.starts_with(|c: char| c.is_ascii_digit() | c.is_ascii_lowercase()) {
            return false;
        }

        let s = s.strip_suffix('s').unwrap_or(s);

        s.chars().all(char::is_alphanumeric)
            && s.chars().filter(|&c| c.is_uppercase()).take(2).count() > 1
            && s.chars().filter(|&c| c.is_lowercase()).take(1).count() > 0
    }

    fn has_underscore(s: &str) -> bool {
        s != "_" && !s.contains("\\_") && s.contains('_')
    }

    fn has_hyphen(s: &str) -> bool {
        s != "-" && s.contains('-')
    }

    if let Ok(url) = Url::parse(word) {
        // try to get around the fact that `foo::bar` parses as a valid URL
        if !url.cannot_be_a_base() {
            span_lint(
                cx,
                DOC_MARKDOWN,
                span,
                "you should put bare URLs between `<`/`>` or make a proper Markdown link",
            );

            return;
        }
    }

    // We assume that mixed-case words are not meant to be put inside backticks. (Issue #2343)
    if has_underscore(word) && has_hyphen(word) {
        return;
    }

    if has_underscore(word) || word.contains("::") || is_camel_case(word) || word.ends_with("()") {
        let mut applicability = Applicability::MachineApplicable;

        span_lint_and_then(
            cx,
            DOC_MARKDOWN,
            span,
            "item in documentation is missing backticks",
            |diag| {
                let snippet = snippet_with_applicability(cx, span, "..", &mut applicability);
                diag.span_suggestion_with_style(
                    span,
                    "try",
                    format!("`{snippet}`"),
                    applicability,
                    // always show the suggestion in a separate line, since the
                    // inline presentation adds another pair of backticks
                    SuggestionStyle::ShowAlways,
                );
            },
        );
    }
}

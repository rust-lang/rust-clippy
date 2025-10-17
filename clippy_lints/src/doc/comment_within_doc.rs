use rustc_ast::token::CommentKind;
use rustc_ast::{AttrKind, AttrStyle};
use rustc_errors::Applicability;
use rustc_lexer::{TokenKind, tokenize};
use rustc_lint::{EarlyContext, LintContext};
use rustc_span::source_map::SourceMap;
use rustc_span::{BytePos, Span};

use clippy_utils::diagnostics::span_lint_and_then;

use super::COMMENT_WITHIN_DOC;

struct AttrInfo {
    line: usize,
    is_outer: bool,
    span: Span,
    file_span_pos: BytePos,
}

impl AttrInfo {
    fn new(source_map: &SourceMap, attr: &rustc_ast::Attribute) -> Option<Self> {
        let span_info = source_map.span_to_lines(attr.span).ok()?;
        // If we cannot get the line for any reason, no point in building this item.
        let line = span_info.lines.last()?.line_index;
        Some(Self {
            line,
            is_outer: attr.style == AttrStyle::Outer,
            span: attr.span,
            file_span_pos: span_info.file.start_pos,
        })
    }
}

// Returns a `Vec` of `TokenKind` if the span only contains comments, otherwise returns `None`.
fn snippet_contains_only_comments(snippet: &str) -> Option<Vec<TokenKind>> {
    let mut tokens = Vec::new();
    for token in tokenize(snippet) {
        match token.kind {
            TokenKind::Whitespace => {},
            TokenKind::BlockComment { .. } | TokenKind::LineComment { .. } => tokens.push(token.kind),
            _ => return None,
        }
    }
    Some(tokens)
}

pub(super) fn check(cx: &EarlyContext<'_>, attrs: &[rustc_ast::Attribute]) {
    let mut stored_prev_attr = None;
    let source_map = cx.sess().source_map();
    for attr in attrs
        .iter()
        // We ignore `#[doc = "..."]` and `/** */` attributes.
        .filter(|attr| matches!(attr.kind, AttrKind::DocComment(CommentKind::Line, _)))
    {
        let Some(attr) = AttrInfo::new(source_map, attr) else {
            stored_prev_attr = None;
            continue;
        };
        let Some(ref prev_attr) = stored_prev_attr else {
            stored_prev_attr = Some(attr);
            continue;
        };
        // First we check if they are from the same file and if they are the same kind of doc
        // comments.
        if attr.file_span_pos != prev_attr.file_span_pos || attr.is_outer != prev_attr.is_outer {
            stored_prev_attr = Some(attr);
            continue;
        }
        let Some(nb_lines) = attr.line.checked_sub(prev_attr.line + 1) else {
            stored_prev_attr = Some(attr);
            continue;
        };
        // Then we check if they follow each other.
        if nb_lines == 0 || nb_lines > 1 {
            // If there is no line between them or there are more than 1, we skip this check.
            stored_prev_attr = Some(attr);
            continue;
        }
        let span_between = prev_attr.span.between(attr.span);
        // If there is one line between the two doc comments and this line contains a line code
        // comment, then we lint.
        if nb_lines == 1
            && let Ok(snippet) = source_map.span_to_snippet(span_between)
            && let Some(comments) = snippet_contains_only_comments(&snippet)
            && let &[TokenKind::LineComment { .. }] = comments.as_slice()
        {
            let offset_begin = snippet.len() - snippet.trim_start().len();
            let offset_end = snippet.len() - snippet.trim_end().len();
            let span = span_between
                .with_lo(span_between.lo() + BytePos(offset_begin.try_into().unwrap()))
                .with_hi(span_between.hi() - BytePos(offset_end.try_into().unwrap()));
            let comment_kind = if attr.is_outer { '/' } else { '!' };
            span_lint_and_then(
                cx,
                COMMENT_WITHIN_DOC,
                vec![prev_attr.span, span, attr.span],
                "code comment surrounded by doc comments",
                |diag| {
                    diag.span_suggestion(
                        span.with_hi(span.lo() + BytePos(2)),
                        "did you mean to make it a doc comment?",
                        format!("//{comment_kind}"),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
        stored_prev_attr = Some(attr);
    }
}

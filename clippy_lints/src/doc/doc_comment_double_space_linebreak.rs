use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::DOC_COMMENT_DOUBLE_SPACE_LINEBREAK;

pub fn check(cx: &LateContext<'_>, collected_breaks: &[Span]) {
    let replacements: Vec<_> = collect_doc_replacements(cx, collected_breaks);

    if let Some((&(lo_span, _), &(hi_span, _))) = replacements.first().zip(replacements.last()) {
        span_lint_and_then(
            cx,
            DOC_COMMENT_DOUBLE_SPACE_LINEBREAK,
            lo_span.to(hi_span),
            "doc comments should use a back-slash (\\) instead of a double space to indicate a linebreak",
            |diag| {
                diag.multipart_suggestion(
                    "replace this double space with a back-slash",
                    replacements,
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

fn collect_doc_replacements(cx: &LateContext<'_>, spans: &[Span]) -> Vec<(Span, String)> {
    spans
        .iter()
        .map(|span| {
            let s = snippet(cx, *span, "..");
            let after_newline = s.trim_start_matches(' ');

            let new_comment = format!("\\{after_newline}");
            (*span, new_comment)
        })
        .collect()
}

/*
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_ast::token::CommentKind;
use rustc_ast::{AttrKind, AttrStyle, Attribute};
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::DOC_COMMENT_DOUBLE_SPACE_LINEBREAK;

pub fn check(cx: &LateContext<'_>, attrs: &[Attribute]) {
    let replacements: Vec<_> = collect_doc_replacements(attrs);

    if let Some((&(lo_span, _), &(hi_span, _))) = replacements.first().zip(replacements.last()) {
        span_lint_and_then(
            cx,
            DOC_COMMENT_DOUBLE_SPACE_LINEBREAK,
            lo_span.to(hi_span),
            "doc comments should use a back-slash (\\) instead of a double space to indicate a linebreak",
            |diag| {
                diag.multipart_suggestion(
                    "replace this double space with a back-slash",
                    replacements,
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

fn collect_doc_replacements(attrs: &[Attribute]) -> Vec<(Span, String)> {
    attrs
        .iter()
        .filter_map(|attr| {
            if let AttrKind::DocComment(com_kind, sym) = attr.kind
                && !attr.span.from_expansion()
                && com_kind == CommentKind::Line
                && let comment = sym.as_str()
                && comment.ends_with("  ")
            {
                let pre = match attr.style {
                    AttrStyle::Outer => "///",
                    AttrStyle::Inner => "//!",
                };

                let len = comment.len();
                let new_comment = format!("{pre}{}\\", &comment[..len - 2]);
                Some((attr.span, new_comment))
            } else {
                None
            }
        })
        .collect()
}
*/

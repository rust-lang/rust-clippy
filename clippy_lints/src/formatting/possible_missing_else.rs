use super::POSSIBLE_MISSING_ELSE;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{FileRangeExt, SpanExt};
use rustc_ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;
use rustc_span::SyntaxContext;

pub(super) fn check(cx: &EarlyContext<'_>, ctxt: SyntaxContext, first: &Expr, second: &Expr) {
    if matches!(first.kind, ExprKind::If(..))
        && matches!(second.kind, ExprKind::If(..) | ExprKind::Block(..))
        && let first_data = first.span.data()
        && let second_data = second.span.data()
        && first_data.ctxt == ctxt
        && second_data.ctxt == ctxt
        && let Some((scx, range)) = first_data.mk_edit_cx(cx)
        && scx
            .get_text(range.clone())
            .is_some_and(|src| src.starts_with("if") && src.ends_with('}'))
        && let Some(range) = range.get_range_between(&scx, second_data)
        && scx
            .get_text(range.clone())
            .is_some_and(|src| src.chars().all(|c| c != '\n' && c.is_whitespace()))
        && let Some(indent) = scx.get_line_indent_before(range.start)
    {
        let lint_sp = scx.mk_span(range);
        span_lint_and_then(
            cx,
            POSSIBLE_MISSING_ELSE,
            lint_sp,
            "this is formatted as though there should be an `else`",
            |diag| {
                diag.span_suggestion(lint_sp, "add an `else`", " else ", Applicability::MaybeIncorrect)
                    .span_suggestion(
                        lint_sp,
                        "add a line break",
                        format!("\n{indent}"),
                        Applicability::MaybeIncorrect,
                    );
            },
        );
    }
}

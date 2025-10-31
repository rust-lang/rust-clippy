use super::POSSIBLE_MISSING_COMMA;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{FileRangeExt, SpanExt};
use rustc_ast::{BinOpKind, Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;
use rustc_span::{Span, SyntaxContext};

pub(super) fn check(cx: &EarlyContext<'_>, ctxt: SyntaxContext, e: &Expr) {
    if let ExprKind::Binary(op, lhs, rhs) = &e.kind
        && let e_data = e.span.data()
        && e_data.ctxt == ctxt
    {
        if matches!(
            op.node,
            BinOpKind::And | BinOpKind::Mul | BinOpKind::Sub | BinOpKind::BitAnd
        ) && let op_data = op.span.data()
            && op_data.ctxt == e_data.ctxt
            && let Some(insert_sp) = op_data.map_range(cx, |scx, range| {
                range
                    .extend_end_to(scx, e_data.hi_ctxt())
                    .filter(|range| {
                        scx.get_text(..range.start)
                            .is_some_and(|src| src.ends_with(char::is_whitespace))
                            && scx
                                .get_text(range.clone())
                                .and_then(|src| src.strip_prefix(op.node.as_str()))
                                .is_some_and(|src| src.starts_with(|c: char| !c.is_whitespace() && c != '/'))
                    })?
                    .with_leading_whitespace(scx)
                    .map(|range| range.start..range.start)
            })
            && let Some(insert_sp) = match lhs.span.walk_to_ctxt(ctxt) {
                Some(lhs_sp) => {
                    let lhs_data = lhs_sp.data();
                    // Sanity check that the lhs actually comes first.
                    (lhs_data.hi <= insert_sp.hi())
                        .then(|| Span::new(lhs_data.hi, lhs_data.hi, lhs_data.ctxt, lhs_data.parent))
                },
                None => Some(insert_sp),
            }
        {
            span_lint_and_then(
                cx,
                POSSIBLE_MISSING_COMMA,
                op.span,
                "the is formatted like a unary operator, but it's parsed as a binary operator",
                |diag| {
                    diag.span_suggestion(insert_sp, "add a comma before", ",", Applicability::MaybeIncorrect)
                        .span_suggestion(
                            Span::new(op_data.hi, op_data.hi, op_data.ctxt, op_data.parent),
                            "add a space after",
                            " ",
                            Applicability::MaybeIncorrect,
                        );
                },
            );
        }
        check(cx, ctxt, lhs);
        check(cx, ctxt, rhs);
    }
}

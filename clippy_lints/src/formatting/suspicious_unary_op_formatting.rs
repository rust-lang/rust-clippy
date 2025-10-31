use super::SUSPICIOUS_UNARY_OP_FORMATTING;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{FileRangeExt, SpanExt, StrExt};
use rustc_ast::{BinOp, Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, LintContext};

pub(super) fn check(cx: &EarlyContext<'_>, bin_expr: &Expr, bin_op: &BinOp, rhs: &Expr) {
    if let ExprKind::Unary(un_op, _) = rhs.kind
        && let bin_op_data = bin_op.span.data()
        && bin_op_data.ctxt == bin_expr.span.ctxt()
        && let rhs_data = rhs.span.data()
        && rhs_data.ctxt == bin_op_data.ctxt
        && let bin_op_str = bin_op.node.as_str()
        && let un_op_str = un_op.as_str()
        && let sm = cx.sess().source_map()
        && !bin_op_data.ctxt.in_external_macro(sm)
        && let Some([lint_sp, sugg_sp]) = bin_op_data.map_range(sm, |scx, range| {
            let lint_range = range
                .extend_end_to(scx, rhs_data.hi_ctxt())?
                .map_range_text(scx, |src| {
                    src.split_multipart_prefix([bin_op_str, un_op_str])
                        .and_then(|[s, rest]| rest.starts_with(char::is_whitespace).then_some(s))
                })?;
            lint_range
                .clone()
                .with_trailing_whitespace(scx)
                .map(|sugg_range| [lint_range, sugg_range])
        })
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_UNARY_OP_FORMATTING,
            lint_sp,
            "this formatting makes the binary and unary operators look like a single operator",
            |diag| {
                diag.span_suggestion(
                    sugg_sp,
                    "add a space between",
                    format!("{bin_op_str} {un_op_str}"),
                    if bin_op_data.ctxt.is_root() {
                        Applicability::MachineApplicable
                    } else {
                        Applicability::MaybeIncorrect
                    },
                );
            },
        );
    }
}

use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

pub fn check(cx: &LateContext<'_>, expr: &Expr<'_>, span: Span) {
    clippy_utils::diagnostics::span_lint(
        cx,
        super::PARSE_TO_STRING,
        expr.span.with_lo(span.lo()),
        "parsing a the output of `.to_string()`",
    );
}

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{trim_span, walk_span_to_context};
use rustc_ast::ast::{Expr, ExprKind, LitKind, Pat, PatKind, RangeEnd, RangeLimits};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for ranges which almost include the entire range of letters from 'a' to 'z'
    /// or digits from '0' to '9', but don't because they're a half open range.
    ///
    /// ### Why is this bad?
    /// This (`'a'..'z'`) is almost certainly a typo meant to include all letters.
    ///
    /// ### Example
    /// ```rust
    /// let _ = 'a'..'z';
    /// ```
    /// Use instead:
    /// ```rust
    /// let _ = 'a'..='z';
    /// ```
    #[clippy::version = "1.68.0"]
    pub ALMOST_COMPLETE_RANGE,
    suspicious,
    "almost complete range"
}
impl_lint_pass!(AlmostCompleteRange => [ALMOST_COMPLETE_RANGE]);

pub struct AlmostCompleteRange {
    msrv: Msrv,
}
impl AlmostCompleteRange {
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}
impl EarlyLintPass for AlmostCompleteRange {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, e: &Expr) {
        if let ExprKind::Range(Some(start), Some(end), RangeLimits::HalfOpen) = &e.kind {
            let ctxt = e.span.ctxt();
            let sugg = if let Some(start) = walk_span_to_context(start.span, ctxt)
                && let Some(end) = walk_span_to_context(end.span, ctxt)
                && self.msrv.meets(msrvs::RANGE_INCLUSIVE)
            {
                Some((trim_span(cx.sess().source_map(), start.between(end)), "..="))
            } else {
                None
            };
            check_range(cx, e.span, start, end, sugg);
        }
    }

    fn check_pat(&mut self, cx: &EarlyContext<'_>, p: &Pat) {
        if let PatKind::Range(Some(start), Some(end), kind) = &p.kind
            && matches!(kind.node, RangeEnd::Excluded)
        {
            let sugg = if self.msrv.meets(msrvs::RANGE_INCLUSIVE) {
                "..="
            } else {
                "..."
            };
            check_range(cx, p.span, start, end, Some((kind.span, sugg)));
        }
    }

    extract_msrv_attr!(EarlyContext);
}

fn check_range(cx: &EarlyContext<'_>, span: Span, start: &Expr, end: &Expr, sugg: Option<(Span, &str)>) {
    if let ExprKind::Lit(start_token_lit) = start.peel_parens().kind
        && let ExprKind::Lit(end_token_lit) = end.peel_parens().kind
        && matches!(
            (
                LitKind::from_token_lit(start_token_lit),
                LitKind::from_token_lit(end_token_lit),
            ),
            (
                Ok(LitKind::Byte(b'a') | LitKind::Char('a')),
                Ok(LitKind::Byte(b'z') | LitKind::Char('z'))
            )
            | (
                Ok(LitKind::Byte(b'A') | LitKind::Char('A')),
                Ok(LitKind::Byte(b'Z') | LitKind::Char('Z')),
            )
            | (
                Ok(LitKind::Byte(b'0') | LitKind::Char('0')),
                Ok(LitKind::Byte(b'9') | LitKind::Char('9')),
            )
        )
        && !in_external_macro(cx.sess(), span)
    {
        span_lint_and_then(
            cx,
            ALMOST_COMPLETE_RANGE,
            span,
            "almost complete ascii range",
            |diag| {
                if let Some((span, sugg)) = sugg {
                    diag.span_suggestion(
                        span,
                        "use an inclusive range",
                        sugg,
                        Applicability::MaybeIncorrect,
                    );
                }
            }
        );
    }
}

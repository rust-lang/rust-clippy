use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::SpanRangeExt;

use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks the usage of `.first().is_some()` or `.first().is_none()` to check if a slice is
    /// empty.
    ///
    /// ### Why is this bad?
    /// Using `.is_empty()` is shorter and better communicates the intention.
    ///
    /// ### Example
    /// ```no_run
    /// let v = vec![1, 2, 3];
    /// if v.first().is_none() {
    ///     // The vector is empty...
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let v = vec![1, 2, 3];
    /// if v.is_empty() {
    ///     // The vector is empty...
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub UNNECESSARY_FIRST_THEN_CHECK,
    complexity,
    "calling `.first().is_some()` or `.first().is_none()` instead of `.is_empty()`"
}

pub(super) fn check(
    cx: &LateContext<'_>,
    call_span: Span,
    first_call: &Expr<'_>,
    first_caller: &Expr<'_>,
    is_some: bool,
) {
    if !cx
        .typeck_results()
        .expr_ty_adjusted(first_caller)
        .peel_refs()
        .is_slice()
    {
        return;
    }

    let ExprKind::MethodCall(_, _, _, first_call_span) = first_call.kind else {
        return;
    };

    let both_calls_span = first_call_span.with_hi(call_span.hi());
    if let Some(both_calls_snippet) = both_calls_span.get_source_text(cx)
        && let Some(first_caller_snippet) = first_caller.span.get_source_text(cx)
    {
        let (sugg_span, suggestion) = if is_some {
            (
                first_caller.span.with_hi(call_span.hi()),
                format!("!{first_caller_snippet}.is_empty()"),
            )
        } else {
            (both_calls_span, "is_empty()".to_owned())
        };
        span_lint_and_sugg(
            cx,
            UNNECESSARY_FIRST_THEN_CHECK,
            sugg_span,
            format!(
                "unnecessary use of `{both_calls_snippet}` to check if slice {}",
                if is_some { "is not empty" } else { "is empty" }
            ),
            "replace this with",
            suggestion,
            Applicability::MachineApplicable,
        );
    }
}

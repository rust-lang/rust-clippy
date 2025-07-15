use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::{is_expr_identity_function, is_trait_method};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.map(…)`, followed by `.all(identity)` or `.any(identity)`.
    ///
    /// ### Why is this bad?
    /// The `.all(…)` or `.any(…)` methods can be called directly in place of `.map(…)`.
    ///
    /// ### Example
    /// ```
    /// # let mut v = [""];
    /// let e1 = v.iter().map(|s| s.is_empty()).all(|a| a);
    /// let e2 = v.iter().map(|s| s.is_empty()).any(std::convert::identity);
    /// ```
    /// Use instead:
    /// ```
    /// # let mut v = [""];
    /// let e1 = v.iter().all(|s| s.is_empty());
    /// let e2 = v.iter().any(|s| s.is_empty());
    /// ```
    #[clippy::version = "1.84.0"]
    pub MAP_ALL_ANY_IDENTITY,
    complexity,
    "combine `.map(_)` followed by `.all(identity)`/`.any(identity)` into a single call"
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    recv: &Expr<'_>,
    map_call_span: Span,
    map_arg: &Expr<'_>,
    any_call_span: Span,
    any_arg: &Expr<'_>,
    method: &str,
) {
    if is_trait_method(cx, expr, sym::Iterator)
        && is_trait_method(cx, recv, sym::Iterator)
        && is_expr_identity_function(cx, any_arg)
        && let map_any_call_span = map_call_span.with_hi(any_call_span.hi())
        && let Some(map_arg) = map_arg.span.get_source_text(cx)
    {
        span_lint_and_then(
            cx,
            MAP_ALL_ANY_IDENTITY,
            map_any_call_span,
            format!("usage of `.map(...).{method}(identity)`"),
            |diag| {
                diag.span_suggestion_verbose(
                    map_any_call_span,
                    format!("use `.{method}(...)` instead"),
                    format!("{method}({map_arg})"),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::{is_expr_untyped_identity_function, is_trait_method};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `flat_map(|x| x)`.
    ///
    /// ### Why is this bad?
    /// Readability, this can be written more concisely by using `flatten`.
    ///
    /// ### Example
    /// ```no_run
    /// # let iter = vec![vec![0]].into_iter();
    /// iter.flat_map(|x| x);
    /// ```
    /// Can be written as
    /// ```no_run
    /// # let iter = vec![vec![0]].into_iter();
    /// iter.flatten();
    /// ```
    #[clippy::version = "1.39.0"]
    pub FLAT_MAP_IDENTITY,
    complexity,
    "call to `flat_map` where `flatten` is sufficient"
}

/// lint use of `flat_map` for `Iterators` where `flatten` would be sufficient
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    flat_map_arg: &'tcx hir::Expr<'_>,
    flat_map_span: Span,
) {
    if is_trait_method(cx, expr, sym::Iterator) && is_expr_untyped_identity_function(cx, flat_map_arg) {
        span_lint_and_sugg(
            cx,
            FLAT_MAP_IDENTITY,
            flat_map_span.with_hi(expr.span.hi()),
            "use of `flat_map` with an identity function",
            "try",
            "flatten()".to_string(),
            Applicability::MachineApplicable,
        );
    }
}

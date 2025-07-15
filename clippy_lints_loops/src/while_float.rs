use clippy_utils::diagnostics::span_lint;
use rustc_hir::ExprKind;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for while loops comparing floating point values.
    ///
    /// ### Why is this bad?
    /// If you increment floating point values, errors can compound,
    /// so, use integers instead if possible.
    ///
    /// ### Known problems
    /// The lint will catch all while loops comparing floating point
    /// values without regarding the increment.
    ///
    /// ### Example
    /// ```no_run
    /// let mut x = 0.0;
    /// while x < 42.0 {
    ///     x += 1.0;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut x = 0;
    /// while x < 42 {
    ///     x += 1;
    /// }
    /// ```
    #[clippy::version = "1.80.0"]
    pub WHILE_FLOAT,
    nursery,
    "while loops comparing floating point values"
}

pub(super) fn check(cx: &rustc_lint::LateContext<'_>, condition: &rustc_hir::Expr<'_>) {
    if let ExprKind::Binary(_op, left, right) = condition.kind
        && is_float_type(cx, left)
        && is_float_type(cx, right)
    {
        span_lint(cx, WHILE_FLOAT, condition.span, "while condition comparing floats");
    }
}

fn is_float_type(cx: &rustc_lint::LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    cx.typeck_results().expr_ty(expr).is_floating_point()
}

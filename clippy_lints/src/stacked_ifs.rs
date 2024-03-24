use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::in_constant;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects `if` statements where the condition is another `if` statement.
    /// ### Why is this bad?
    /// This makes code hard to read.
    /// ### Example
    /// ```no_run
    /// if if some_condition {
    ///     some_value
    /// } else {
    ///     some_other_value
    /// } == another_value {
    ///     // Do something.
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let value = if some_condition {
    ///     some_value
    /// } else {
    ///     some_other_value
    /// };
    /// if value == another_value {
    ///     // Do something.
    /// }
    /// ```
    #[clippy::version = "1.79.0"]
    pub STACKED_IFS,
    style,
    "default lint description"
}

declare_lint_pass!(StackedIfs => [STACKED_IFS]);

impl LateLintPass<'_> for StackedIfs {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        // Ensure that the expression isn't a part of a constant or macro expansion.
        if expr.span.from_expansion() || in_constant(cx, expr.hir_id) {
            return;
        }
        stacked_ifs(cx, expr);
    }
}

fn stacked_ifs(cx: &LateContext<'_>, expr: &Expr<'_>) {
    // Check for if statements where the condition is another if statement.
    let ExprKind::If(condition, _, _) = expr.kind else {
        return;
    };

    // Remove any DropTemps wrapping the `if` statement.
    let condition = match condition.kind {
        ExprKind::DropTemps(expr) => expr,
        _ => condition,
    };

    let condition = match condition.kind {
        ExprKind::Binary(_, expr, _) => expr,
        _ => condition,
    };

    if let ExprKind::If(_, _, _) = condition.kind {
        span_lint_and_help(
            cx,
            STACKED_IFS,
            expr.span,
            "Stacked `if` found",
            None,
            "Avoid using an `if` statement as a condition for another `if` statement.",
        );
    }
}

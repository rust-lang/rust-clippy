use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects `if` expressions where the condition is another `if` expression.
    /// ### Why is this bad?
    /// This makes code hard to read.
    /// ### Example
    /// ```no_run
    /// let a = 3;
    /// let b = 4;
    /// let c = 5;
    /// if if a == b {
    ///     4
    /// } else {
    ///     5
    /// } == c {
    ///     // Do something.
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = 3;
    /// let b = 4;
    /// let c = 5;
    /// let value = if a == b {
    ///     4
    /// } else {
    ///     5
    /// };
    /// if value == c {
    ///     // Do something.
    /// }
    /// ```
    #[clippy::version = "1.79.0"]
    pub STACKED_IFS,
    style,
    "finds if expressions with another if expression as the condition"
}

declare_lint_pass!(StackedIfs => [STACKED_IFS]);

impl LateLintPass<'_> for StackedIfs {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        // Ensure that the expression isn't a part of a macro expansion.
        if !expr.span.from_expansion() {
            stacked_ifs(cx, expr);
        }
    }
}

fn stacked_ifs(cx: &LateContext<'_>, expr: &Expr<'_>) {
    // Check for if expressions where the condition is another if expression.
    let ExprKind::If(condition, _, _) = expr.kind else {
        return;
    };

    let condition = condition.peel_drop_temps();

    // Do not lint if condition is from a macro expansion.
    if condition.span.from_expansion() {
        return;
    }

    if let ExprKind::If(..) = condition.kind {
        emit_lint(cx, condition);
    }

    if let ExprKind::Binary(_, lhs, rhs) = condition.kind {
        if let ExprKind::If(..) = lhs.kind
            && !lhs.span.from_expansion()
        {
            emit_lint(cx, lhs);
        }
        if let ExprKind::If(..) = rhs.kind
            && !rhs.span.from_expansion()
        {
            emit_lint(cx, rhs);
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>) {
    span_lint_and_help(
        cx,
        STACKED_IFS,
        expr.span,
        "stacked `if` found",
        None,
        "avoid using an `if` expression as a condition for another `if` expression",
    );
}

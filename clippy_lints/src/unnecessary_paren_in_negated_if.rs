use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary parentheses in negated if conditions.
    ///
    /// ### Why is this bad?
    /// Unnecessary parentheses make code harder to read and add visual noise.
    ///
    /// ### Example
    /// ```rust
    /// if !(condition) {
    ///     // ...
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// if !condition {
    ///     // ...
    /// }
    /// ```
    #[clippy::version = "1.75.0"]
    pub UNNECESSARY_PAREN_IN_NEGATED_IF,
    style,
    "unnecessary parentheses in negated if conditions"
}

declare_lint_pass!(UnnecessaryParenInNegatedIf => [UNNECESSARY_PAREN_IN_NEGATED_IF]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryParenInNegatedIf {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::If(cond, ..) = expr.kind {
            // Check if the condition is a negation
            if let ExprKind::Unary(UnOp::Not, inner) = cond.kind {
                // Check if the inner expression is parenthesized by examining the source code
                let mut applicability = Applicability::MachineApplicable;
                let inner_snippet = snippet_with_applicability(cx, inner.span, "..", &mut applicability);

                // If the snippet starts with '(' and ends with ')', it's likely parenthesized
                if inner_snippet.starts_with('(') && inner_snippet.ends_with(')') {
                    // Extract the content inside the parentheses
                    let content = &inner_snippet[1..inner_snippet.len() - 1];

                    // Don't lint if the expression is from a macro
                    if expr.span.from_expansion() {
                        return;
                    }

                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_PAREN_IN_NEGATED_IF,
                        inner.span,
                        "unnecessary parentheses in negated if condition",
                        "try",
                        format!("!{content}"),
                        applicability,
                    );
                }
            }
        }
    }
}

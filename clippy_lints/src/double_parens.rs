use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{HasSession, snippet_with_applicability};
use rustc_ast::ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary double parentheses.
    ///
    /// ### Why is this bad?
    /// This makes code harder to read and might indicate a
    /// mistake.
    ///
    /// ### Example
    /// ```no_run
    /// fn simple_double_parens() -> i32 {
    ///     ((0))
    /// }
    ///
    /// # fn foo(bar: usize) {}
    /// foo((0));
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn simple_no_parens() -> i32 {
    ///     0
    /// }
    ///
    /// # fn foo(bar: usize) {}
    /// foo(0);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub DOUBLE_PARENS,
    complexity,
    "Warn on unnecessary double parentheses"
}

declare_lint_pass!(DoubleParens => [DOUBLE_PARENS]);

impl EarlyLintPass for DoubleParens {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        let (outer_span, inner_span) = match &expr.kind {
            ExprKind::Paren(in_paren) => {
                let inner_span = match &in_paren.kind {
                    ExprKind::Paren(inner) => inner.span,
                    ExprKind::Tup(_) => in_paren.span,
                    _ => return,
                };
                (expr.span, inner_span)
            },
            ExprKind::Call(_, params)
                if let [param] = &**params
                    && let ExprKind::Paren(inner) = &param.kind =>
            {
                (param.span, inner.span)
            },
            ExprKind::MethodCall(call)
                if let [arg] = &*call.args
                    && let ExprKind::Paren(inner) = &arg.kind =>
            {
                (arg.span, inner.span)
            },
            _ => return,
        };
        if !expr.span.from_expansion() {
            let mut applicability = Applicability::MachineApplicable;
            let sugg = snippet_with_applicability(cx.sess(), inner_span, "_", &mut applicability);
            span_lint_and_sugg(
                cx,
                DOUBLE_PARENS,
                outer_span,
                "unnecessary parentheses",
                "remove them",
                sugg.to_string(),
                applicability,
            );
        }
    }
}

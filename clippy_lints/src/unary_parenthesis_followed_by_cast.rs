use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Expr, ExprKind, Path};
use rustc_ast::ast_traits::AstDeref;
use rustc_ast::ptr::P;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for cast which argument is parenthesized variable.
    ///
    /// ### Why is this bad?
    /// It's same effect as `variable as Type`, thus you don't need parentheses.
    ///
    /// ### Example
    /// ```rust
    /// fn no_op(arg_1: f64) {}
    ///
    /// let x = (1.0f32) as f64;
    /// let y = (2.0f32) as f64;
    /// no_op(y);
    /// ```
    /// Use instead:
    /// ```rust
    /// fn no_op(arg_1: f64) {}
    ///
    /// let x = 1.0f32 as f64;
    /// let y = 2.0f32 as f64;
    /// no_op(y);
    /// ```
    #[clippy::version = "1.70.0"]
    pub UNARY_PARENTHESIS_FOLLOWED_BY_CAST,
    complexity,
    "`as` cast with parenthesized simple argument"
}
declare_lint_pass!(UnaryParenthesisFollowedByCast => [UNARY_PARENTHESIS_FOLLOWED_BY_CAST]);

impl EarlyLintPass for UnaryParenthesisFollowedByCast {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let ExprKind::Cast(ref expr, _) = expr.kind
            && let ExprKind::Paren(ref parenthesized) = expr.kind
            && is_item_path_is_local_and_not_qualified(parenthesized)
        {
            span_lint_and_help(
                cx,
                UNARY_PARENTHESIS_FOLLOWED_BY_CAST,
                expr.span,
                "unnecessary parenthesis",
                None,
                "consider remove parenthesis"
            );
        }
    }
}

fn is_item_path_is_local_and_not_qualified(parenthesized: &P<Expr>) -> bool {
    if let ExprKind::Path(ref impl_qualifier, ref item_path) = parenthesized.ast_deref().kind
        && impl_qualifier.is_none()
        // is item_path local variable?
        && !item_path.is_global()
        && let Path { segments, .. } = item_path
        && segments.len() == 1 {
        true
    } else {
        false
    }
}

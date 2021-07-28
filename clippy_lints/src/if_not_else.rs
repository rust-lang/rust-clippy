//! lint on if branches that could be swapped so no `!` operation is necessary
//! on the condition

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `!` or `!=` in an if condition with an
    /// else branch.
    ///
    /// ### Why is this bad?
    /// Negations reduce the readability of statements.
    ///
    /// ### Example
    /// ```rust
    /// # let v: Vec<usize> = vec![];
    /// # fn a() {}
    /// # fn b() {}
    /// if !v.is_empty() {
    ///     a()
    /// } else {
    ///     b()
    /// }
    /// ```
    ///
    /// Could be written:
    ///
    /// ```rust
    /// # let v: Vec<usize> = vec![];
    /// # fn a() {}
    /// # fn b() {}
    /// if v.is_empty() {
    ///     b()
    /// } else {
    ///     a()
    /// }
    /// ```
    pub IF_NOT_ELSE,
    pedantic,
    "`if` branches that could be swapped so no negation operation is necessary on the condition"
}

declare_lint_pass!(IfNotElse => [IF_NOT_ELSE]);

impl EarlyLintPass for IfNotElse {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, item: &Expr) {
        if in_external_macro(cx.sess(), item.span) {
            return;
        }
        if let ExprKind::If(ref cond, _, Some(ref els)) = item.kind {
            if let ExprKind::Block(..) = els.kind {
                match cond.kind {
                    ExprKind::Unary(UnOp::Not, _) => {
                        span_lint_and_help(
                            cx,
                            IF_NOT_ELSE,
                            item.span,
                            "unnecessary boolean `not` operation",
                            None,
                            "remove the `!` and swap the blocks of the `if`/`else`",
                        );
                    },
                    ExprKind::Binary(ref kind, _, _) if kind.node == BinOpKind::Ne => {
                        span_lint_and_help(
                            cx,
                            IF_NOT_ELSE,
                            item.span,
                            "unnecessary `!=` operation",
                            None,
                            "change to `==` and swap the blocks of the `if`/`else`",
                        );
                    },
                    _ => (),
                }
            }
        }
    }
}

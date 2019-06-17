use crate::utils::span_lint_and_sugg;
use if_chain::if_chain;
use rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use rustc::{declare_lint_pass, declare_tool_lint};
use rustc_errors::Applicability;
use syntax::ast::{BinOpKind, Expr, ExprKind, LitKind};

declare_clippy_lint! {
    /// **What it does:** Checks for use of `^` operator when exponentiation was intended.
    ///
    /// **Why is this bad?** This is most probably a typo.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust,ignore
    /// // Bad
    /// 2 ^ 16;
    ///
    /// // Good
    /// 1 << 16;
    /// 2i32.pow(16);
    /// ```
    pub XOR_USED_AS_POW,
    correctness,
    "use of `^` operator when exponentiation was intended"
}

declare_lint_pass!(XorUsedAsPow => [XOR_USED_AS_POW]);

impl EarlyLintPass for XorUsedAsPow {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if_chain! {
            if let ExprKind::Binary(op, left, right) = &expr.node;
            if BinOpKind::BitXor == op.node;
            if let ExprKind::Lit(lit) = &left.node;
            if let LitKind::Int(2, _) = lit.node;
            if let ExprKind::Lit(lit) = &right.node;
            if let LitKind::Int(right, _) = lit.node;
            then {
                span_lint_and_sugg(
                    cx,
                    XOR_USED_AS_POW,
                    expr.span,
                    "`^` is not an exponentiation operator but was used as one",
                    "did you mean to write",
                    format!("1 << {}", right),
                    Applicability::MaybeIncorrect,
                )
            }
        }
    }
}

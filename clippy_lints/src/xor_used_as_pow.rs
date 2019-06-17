use crate::utils::{span_help_and_lint, span_lint_and_sugg};
use if_chain::if_chain;
use rustc::lint::{in_external_macro, EarlyContext, EarlyLintPass, LintArray, LintPass};
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
            if !in_external_macro(cx.sess, expr.span);
            if let ExprKind::Binary(op, left, right) = &expr.node;
            if BinOpKind::BitXor == op.node;
            if let ExprKind::Lit(lit) = &left.node;
            if let LitKind::Int(lhs, _) = lit.node;
            if let ExprKind::Lit(lit) = &right.node;
            if let LitKind::Int(rhs, _) = lit.node;
            then {
                if lhs == 2 {
                    if rhs == 8 || rhs == 16 || rhs == 32 || rhs == 64 || rhs == 128 {
                        span_lint_and_sugg(
                            cx,
                            XOR_USED_AS_POW,
                            expr.span,
                            "it appears you are trying to get the maximum value of an integer, but `^` is not an exponentiation operator",
                            "try",
                            format!("std::u{}::MAX", rhs),
                            Applicability::MaybeIncorrect,
                        )
                    } else {
                        span_lint_and_sugg(
                            cx,
                            XOR_USED_AS_POW,
                            expr.span,
                            "it appears you are trying to get a power of two, but `^` is not an exponentiation operator",
                            "use a bitshift instead",
                            format!("1 << {}", rhs),
                            Applicability::MaybeIncorrect,
                        )
                    }
                } else {
                    span_help_and_lint(
                        cx,
                        XOR_USED_AS_POW,
                        expr.span,
                        "`^` is not an exponentiation operator but appears to have been used as one",
                        "did you mean to use .pow()?"
                    )
                }
            }
        }
    }
}

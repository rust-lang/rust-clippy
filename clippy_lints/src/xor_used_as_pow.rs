use crate::utils::{span_lint_and_help, span_lint_and_sugg};
use if_chain::if_chain;
use rustc_ast::{BinOpKind, Expr, ExprKind, LitKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};

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
            if let ExprKind::Binary(op, left, right) = &expr.kind;
            if BinOpKind::BitXor == op.node;
            if let ExprKind::Lit(lit) = &left.kind;
            if let LitKind::Int(lhs, _) = lit.kind;
            if let ExprKind::Lit(lit) = &right.kind;
            if let LitKind::Int(rhs, _) = lit.kind;
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
                    span_lint_and_help(
                        cx,
                        XOR_USED_AS_POW,
                        expr.span,
                        "`^` is not an exponentiation operator but appears to have been used as one",
                        None,
                        "did you mean to use .pow()?"
                    )
                }
            }
        }
    }
}

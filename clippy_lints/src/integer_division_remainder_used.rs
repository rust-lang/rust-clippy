use clippy_utils::diagnostics::span_lint;
use rustc_ast::BinOpKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of division (`/`) and remainder (`%`) operations
    /// when performed on any integer types using the default `Div` and `Rem` trait implementations.
    ///
    /// ### Why restrict this?
    /// In cryptographic contexts, division can result in timing sidechannel vulnerabilities,
    /// and needs to be replaced with constant-time code instead.
    ///
    /// Most often, the compiler will optimize away any constant-time implementation of
    /// an algorithm. The solution to achieve constant-time operation is often to either implement
    /// a constant-time algorithm in assembly code, or to delegate such operations to a well-known,
    /// audited and / or certified cryptographic library.
    ///
    /// ### Example
    /// ```no_run
    /// let _ = 10 / 2; // This will run faster because it's a division by a multiple of 2.
    /// let _ = 10 / 3; // This will run slower than the above.
    /// ```
    #[clippy::version = "1.79.0"]
    pub INTEGER_DIVISION_REMAINDER_USED,
    restriction,
    "use of disallowed default division and remainder operations"
}

declare_lint_pass!(IntegerDivisionRemainderUsed => [INTEGER_DIVISION_REMAINDER_USED]);

impl LateLintPass<'_> for IntegerDivisionRemainderUsed {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::Binary(op, lhs, rhs) = &expr.kind
            && let BinOpKind::Div | BinOpKind::Rem = op.node
            && let lhs_ty = cx.typeck_results().expr_ty(lhs)
            && let rhs_ty = cx.typeck_results().expr_ty(rhs)
            && let ty::Int(_) | ty::Uint(_) = lhs_ty.peel_refs().kind()
            && let ty::Int(_) | ty::Uint(_) = rhs_ty.peel_refs().kind()
        {
            span_lint(
                cx,
                INTEGER_DIVISION_REMAINDER_USED,
                expr.span.source_callsite(),
                format!("use of {} has been disallowed in this context", op.node.as_str()),
            );
        }
    }
}

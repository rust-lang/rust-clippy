use clippy_utils::diagnostics::span_lint;
use clippy_utils::eq_expr_value;
use rustc_hir::{Arm, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the same condition being checked in a match guard and in the match body
    ///
    /// ### Why is this bad?
    /// This is usually just a typo or a copy and paste error.
    ///
    /// ### Known problems
    /// False negatives: if the condition is an impure function, it could've been called twice on
    /// purpose for its side effects
    ///
    /// ### Example
    /// ```no_run
    /// # let n = 0;
    /// # let a = 3;
    /// # let b = 4;
    /// match n {
    ///     0 if a > b => {
    ///         if a > b {
    ///             return;
    ///         }
    ///     }
    ///     _ => {}
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # let n = 0;
    /// # let a = 3;
    /// # let b = 4;
    /// match n {
    ///     0 if a > b => {
    ///         return;
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub DUPLICATE_MATCH_GUARD,
    nursery,
    "a condition in match body duplicating the match guard"
}
declare_lint_pass!(DuplicateMatchGuard => [DUPLICATE_MATCH_GUARD]);

impl<'tcx> LateLintPass<'tcx> for DuplicateMatchGuard {
    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx Arm<'tcx>) {
        if let Some(guard) = arm.guard
            && let ExprKind::Block(block, _) = arm.body.kind
            && block.stmts.is_empty()
            && let Some(trailing_expr) = block.expr
            && let ExprKind::If(cond, _, None) = trailing_expr.kind
            && eq_expr_value(cx, guard, cond.peel_drop_temps())
        {
            span_lint(cx, DUPLICATE_MATCH_GUARD, cond.span, "condition duplicates match guard");
        }
    }
}

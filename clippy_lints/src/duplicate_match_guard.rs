use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eq_expr_value;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{Arm, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;
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
        let Some(guard) = arm.guard else {
            return;
        };

        let (arm_body_expr, body_has_block) = if let ExprKind::Block(block, _) = arm.body.kind {
            if block.stmts.is_empty()
                && let Some(trailing_expr) = block.expr
            {
                (trailing_expr, true)
            } else {
                // the body contains something other than the `if` clause -- bail out
                return;
            }
        } else {
            (arm.body, false)
        };

        if let ExprKind::If(cond, then, None) = arm_body_expr.kind
            && eq_expr_value(cx, guard, cond.peel_drop_temps())
        {
            let ExprKind::Block(then, _) = then.kind else {
                unreachable!("the `then` expr in `ExprKind::If` is always `ExprKind::Block`")
            };

            // the two expressions may be syntactically different, even if identical
            // semantically -- the user might want to replace the condition in the guard
            // with the one in the body
            let mut applicability = Applicability::MaybeIncorrect;

            let sugg = snippet_with_applicability(cx, then.span, "..", &mut applicability);

            if body_has_block {
                // the common case:
                // ```
                // match 0u32 {
                //     0 if true => {
                //         if true {
                //             return;
                //         }
                //     }
                // }
                // ```
                //
                // suggest removing the `if` _and_ the curlies of the inner brace,
                // since the arm body already has braces
                span_lint_and_sugg(
                    cx,
                    DUPLICATE_MATCH_GUARD,
                    remove_block_curlies(arm_body_expr.span),
                    "condition duplicates match guard",
                    "remove the condition",
                    sugg.to_string(),
                    applicability,
                );
            } else {
                // the uncommon case (rusfmt would add the braces here automatically)
                // ```
                // match 0u32 {
                //     0 if true => if true { return; }
                // }
                // ```
                //
                // suggest removing the `if` but _not_ the curlies of the inner brace,
                // since there are no outer braces coming from the arm body
                span_lint_and_sugg(
                    cx,
                    DUPLICATE_MATCH_GUARD,
                    arm_body_expr.span,
                    "condition duplicates match guard",
                    "remove the condition",
                    sugg.to_string(),
                    applicability,
                );
            }
        }
    }
}

/// remove the curlies from a span describing a block
fn remove_block_curlies(span: Span) -> Span {
    span
}

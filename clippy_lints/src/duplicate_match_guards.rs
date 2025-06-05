use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::source::{
    HasSession, IntoSpan, SpanRangeExt, indent_of, reindent_multiline, snippet_with_applicability, trim_span,
};
use clippy_utils::{eq_expr_value, span_contains_comment};
use rustc_errors::Applicability;
use rustc_hir::{Arm, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::BytePos;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the same condition being checked in a match guard and in the match body
    ///
    /// ### Why is this bad?
    /// This is usually just a typo or a copy and paste error.
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
    #[clippy::version = "1.92.0"]
    pub DUPLICATE_MATCH_GUARDS,
    nursery,
    "a condition in match body duplicating the match guard"
}

declare_lint_pass!(DuplicateMatchGuards => [DUPLICATE_MATCH_GUARDS]);

impl<'tcx> LateLintPass<'tcx> for DuplicateMatchGuards {
    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx Arm<'_>) {
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
            // Make sure that we won't swallow any comments.
            // Be extra conservative and bail out on _any_ comment outside of `then`:
            //
            // <pat> if <guard> => { if <cond> <then> }
            // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^      ^^
            let sm = cx.sess().source_map();
            if span_contains_comment(sm, arm.span.with_hi(then.span.lo()))
                || span_contains_comment(sm, arm.span.with_lo(then.span.hi()))
            {
                return;
            }

            // The two expressions may be syntactically different, even if identical semantically
            // -- the user might want to replace the condition in the guard with the one in the body.
            let mut applicability = Applicability::MaybeIncorrect;

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
                // The arm body already has curlies, so we can remove the ones around `then`

                if !then
                    .span
                    .check_source_text(cx, |s| s.starts_with('{') && s.ends_with('}'))
                {
                    // Despite being a block, `then` somehow does not start and end with curlies.
                    // Bail out just to be sure
                    return;
                }

                let sugg_span = then
                    .span
                    .with_lo(then.span.lo() + BytePos(1))
                    .with_hi(then.span.hi() - BytePos(1));

                let sugg_span = trim_span(sm, sugg_span);

                let sugg = snippet_with_applicability(cx, sugg_span, "..", &mut applicability);

                // Since we remove one level of curlies, we should be able to dedent `then` left one level, so this:
                // ```
                // <pat> if <guard> => {
                //     if <cond> {
                //         then
                //         without
                //         curlies
                //     }
                // }
                // ```
                // becomes this:
                // ```
                // <pat> if <guard> => {
                //     then
                //     without
                //     curlies
                // }
                // ```
                let indent = indent_of(cx, sugg_span);
                let sugg = reindent_multiline(&sugg, true, indent.map(|i| i.saturating_sub(4)));

                span_lint_and_sugg(
                    cx,
                    DUPLICATE_MATCH_GUARDS,
                    arm_body_expr.span,
                    "condition duplicates match guard",
                    "remove the condition",
                    sugg,
                    applicability,
                );
            } else {
                // The uncommon case (rusfmt would add the curlies here automatically)
                // ```
                // match 0u32 {
                //     <pat> if <guard> => if <cond> { return; }
                // }
                // ```
                //
                // the arm body doesn't have its own curlies,
                // so we need to retain the ones around `then`

                let span_to_remove = arm_body_expr
                    .span
                    .with_hi(cond.span.hi())
                    // NOTE: we don't remove trailing whitespace so that there's one space left between `<cond>` and `{`
                    .with_leading_whitespace(cx)
                    .into_span();

                span_lint_and_then(
                    cx,
                    DUPLICATE_MATCH_GUARDS,
                    arm_body_expr.span,
                    "condition duplicates match guard",
                    |diag| {
                        diag.multipart_suggestion_verbose(
                            "remove the condition",
                            vec![(span_to_remove, String::new())],
                            applicability,
                        );
                    },
                );
            }
        }
    }
}

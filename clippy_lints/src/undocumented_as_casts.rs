use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_from_proc_macro;
use clippy_utils::source::text_has_marked_comment;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `as` casts that do not have a preceding `// CAST:` comment.
    ///
    /// ### Why is this bad?
    /// `as` casts are powerful and potentially dangerous. Requiring a documentation comment
    /// ensures the developer has considered the safety and necessity of the conversion.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let x = 0u32 as usize;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // CAST: reason for the cast
    /// let x = 0u32 as usize;
    ///
    /// /* CAST: reason for the cast */
    /// let y = 1u32 as usize;
    /// ```
    #[clippy::version = "1.96.0"]
    pub UNDOCUMENTED_AS_CASTS,
    restriction,
    "`as` casts without a `CAST:` explanation"
}

declare_lint_pass!(UndocumentedAsCasts => [UNDOCUMENTED_AS_CASTS]);

impl<'tcx> LateLintPass<'tcx> for UndocumentedAsCasts {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        let source_map = cx.sess().source_map();
        if let ExprKind::Cast(_, _) = expr.kind
            && !expr.span.in_external_macro(cx.sess().source_map())
            && !is_from_proc_macro(cx, expr)
            && let Ok(line_info) = source_map.lookup_line(expr.span.lo())
            && let Some(src) = line_info.sf.src.as_deref()
            && text_has_marked_comment(
                src,
                &line_info.sf.lines()[..=line_info.line],
                line_info.sf.start_pos,
                "CAST:",
                false,
            )
            .is_none()
        {
            span_lint_and_help(
                cx,
                UNDOCUMENTED_AS_CASTS,
                expr.span,
                "`as` casts without a `// CAST:` explanation",
                None,
                "consider adding a cast comment on the preceding line",
            );
        }
    }
}

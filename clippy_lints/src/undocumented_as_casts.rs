use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{find_preceding_marked_line_comment, first_line_of_span, snippet_indent};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_session::declare_lint_pass;
use rustc_span::{Pos as _, Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `as` casts that do not have a preceding `// CAST:` comment.
    ///
    /// For casts that originate from a declarative or external macro expansion,
    /// the comment must precede the macro call site. Casts from proc-macros are ignored.
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
    /// // CAST: explanation for the cast
    /// let x = 0u32 as usize;
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
        if let ExprKind::Cast(_, _) = expr.kind {
            let from_expansion = expr.span.from_expansion();
            let comment_span = if from_expansion {
                expr.span.source_callsite()
            } else {
                expr.span
            };
            if let Ok(line_info) = source_map.lookup_line(comment_span.lo())
                && let Some(src) = line_info.sf.src.as_deref()
                && find_preceding_marked_line_comment(
                    src,
                    &line_info.sf.lines()[..=line_info.line],
                    line_info.sf.start_pos,
                    "CAST",
                    true,
                )
                .is_none()
            {
                span_lint_and_then(
                    cx,
                    UNDOCUMENTED_AS_CASTS,
                    comment_span,
                    if from_expansion {
                        "found `as` cast from a macro expansion without a `// CAST:` explanation at the call site"
                    } else {
                        "found `as` cast without a `// CAST:` explanation"
                    },
                    |diag| {
                        let (span, sugg) = if from_expansion {
                            let line_start = line_info.sf.lines()[line_info.line].to_usize();
                            let col = (comment_span.lo() - line_info.sf.start_pos).to_usize();
                            let indent = src.get(line_start..col).unwrap_or_default();
                            let sugg_span =
                                Span::new(comment_span.lo(), comment_span.lo(), SyntaxContext::root(), None);
                            (sugg_span, format!("// CAST: <explanation>\n{indent}"))
                        } else {
                            let indent = snippet_indent(cx.sess(), comment_span).unwrap_or_default();
                            (
                                first_line_of_span(cx.sess(), comment_span).shrink_to_lo(),
                                format!("// CAST: <explanation>\n{indent}"),
                            )
                        };
                        diag.span_suggestion(
                            span,
                            "add a cast explanation on the line above",
                            sugg,
                            Applicability::HasPlaceholders,
                        );
                    },
                );
            }
        }
    }
}

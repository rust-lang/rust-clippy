use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_from_proc_macro;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{Pos, Span};

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
    nursery,
    "`as` casts without a `CAST:` explanation"
}

declare_lint_pass!(UndocumentedAsCasts => [UNDOCUMENTED_AS_CASTS]);

impl<'tcx> LateLintPass<'tcx> for UndocumentedAsCasts {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if let ExprKind::Cast(_, _) = expr.kind
            && !expr.span.in_external_macro(cx.sess().source_map())
            && !is_from_proc_macro(cx, expr)
            && !has_preceding_cast_comment(cx, expr.span)
        {
            #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
            span_lint_and_then(
                cx,
                UNDOCUMENTED_AS_CASTS,
                expr.span,
                "`as` casts without a `// CAST:` explanation",
                |diag| {
                    diag.help("consider adding a cast comment on the preceding line");
                },
            );
        }
    }
}

/// Checks if there is a `// CAST:` or `/* CAST:` comment preceding the cast expression.
fn has_preceding_cast_comment(cx: &LateContext<'_>, span: Span) -> bool {
    let source_map = cx.sess().source_map();

    // Try to get the file and line information
    let Ok(line_info) = source_map.lookup_line(span.lo()) else {
        return false;
    };

    let Some(src) = line_info.sf.src.as_deref() else {
        return false;
    };

    let lines = line_info.sf.lines();
    let mut block_comment_end_idx = None;
    let mut found_line_comment = false;

    // Find the preceding lines that start with `//` until hitting a non-comment line
    for line_idx in (0..line_info.line).rev() {
        let start = lines[line_idx].to_usize();
        let end = if line_idx + 1 < lines.len() {
            lines[line_idx + 1].to_usize()
        } else {
            src.len()
        };

        if let Some(prev_text) = src.get(start..end) {
            // Check line comment
            let trimmed = prev_text.trim_start();

            if let Some(description) = trimmed.strip_prefix("//") {
                found_line_comment = true;
                if description.trim().to_ascii_uppercase().starts_with("CAST:") {
                    return true;
                }
                // If it's a comment but not `CAST:`, continue checking previous lines
                continue;
            }

            // Stop if already found line comments but the current line is not a line comment
            if found_line_comment {
                break;
            }

            // Stop if hit non-empty line, but keep track of the end of block comment
            if !trimmed.is_empty() {
                if trimmed.trim_end().ends_with("*/") {
                    block_comment_end_idx = Some(line_idx);
                }
                break;
            }
        }
    }

    // Find `CAST:` in block comments if found the end of a block comment until hitting the start of
    // the block comment
    if let Some(line_end_idx) = block_comment_end_idx {
        for line_idx in (0..=line_end_idx).rev() {
            let start = lines[line_idx].to_usize();
            let end = if line_idx + 1 < lines.len() {
                lines[line_idx + 1].to_usize()
            } else {
                src.len()
            };
            if let Some(prev_text) = src.get(start..end) {
                let trimmed = prev_text.trim_start();

                if trimmed.to_ascii_uppercase().contains("CAST:") {
                    return true;
                }

                // Stop if hit the start of the block comment
                if trimmed.starts_with("/*") {
                    break;
                }
            }
        }
    }

    false
}

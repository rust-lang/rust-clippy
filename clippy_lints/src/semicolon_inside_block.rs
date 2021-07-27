use crate::rustc_lint::LintContext;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::in_macro;
use clippy_utils::source::snippet_with_macro_callsite;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{Block, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** For () returning expressions, check that the semicolon is inside the block.
    ///
    /// **Why is this bad?** For consistency it's best to have the semicolon inside/outside the block. Either way is fine and this lint suggests inside the block.
    /// Take a look at `semicolon_outside_block` for the other alternative.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// unsafe { f(x) };
    /// ```
    /// Use instead:
    /// ```rust
    /// unsafe { f(x); }
    /// ```
    pub SEMICOLON_INSIDE_BLOCK,
    pedantic,
    "add a semicolon inside the block"
}

declare_lint_pass!(SemicolonInsideBlock => [SEMICOLON_INSIDE_BLOCK]);

impl LateLintPass<'_> for SemicolonInsideBlock {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if_chain! {
         if !in_macro(block.span);
         if let Some(expr) = block.expr;
         let t_expr = cx.typeck_results().expr_ty(expr);
         if t_expr.is_unit();
         if let snippet = snippet_with_macro_callsite(cx, expr.span, "}");
         if !snippet.ends_with("};") && !snippet.ends_with('}');
         then {
             // filter out the desugared `for` loop
             if let ExprKind::DropTemps(..) = &expr.kind {
                 return;
             }

             let expr_snip = snippet_with_macro_callsite(cx, expr.span, "..");

             // check for the right suggestion and span, differs if the block spans
             // multiple lines
             let (suggestion, span) = if cx.sess().source_map().is_multiline(block.span) {
                 (format!("{};", expr_snip), expr.span.source_callsite())
            } else {
                let block_with_pot_sem = cx.sess().source_map().span_extend_to_next_char(block.span, '\n', false);
                let block_snip = snippet_with_macro_callsite(cx, block.span, "..");

                (block_snip.replace(expr_snip.as_ref(), &format!("{};", expr_snip)), block_with_pot_sem)
            };

            span_lint_and_sugg(
                cx,
                SEMICOLON_INSIDE_BLOCK,
                span,
                "consider moving the `;` inside the block for consistent formatting",
                "put the `;` here",
                suggestion,
                Applicability::MaybeIncorrect,
            );
         }
        }
    }
}

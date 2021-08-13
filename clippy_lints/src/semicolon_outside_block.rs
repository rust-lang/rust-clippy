use crate::rustc_lint::LintContext;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::in_macro;
use clippy_utils::source::snippet_with_macro_callsite;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::ExprKind;
use rustc_hir::{Block, BodyOwnerKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::BytePos;
use rustc_span::Span;
use std::ops::Add;

declare_clippy_lint! {
    /// **What it does:** For () returning expressions, check that the semicolon is outside the block.
    ///
    /// **Why is this bad?** For consistency it's best to have the semicolon inside/outside the block. Either way is fine and this lint suggests outside the block.
    /// Take a look at `semicolon_inside_block` for the other alternative.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// unsafe { f(x); }
    /// ```
    /// Use instead:
    /// ```rust
    /// unsafe { f(x) };
    /// ```
    pub SEMICOLON_OUTSIDE_BLOCK,
    pedantic,
    "add a semicolon outside the block"
}

declare_lint_pass!(SemicolonOutsideBlock => [SEMICOLON_OUTSIDE_BLOCK]);

impl LateLintPass<'_> for SemicolonOutsideBlock {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if_chain! {
            if !in_macro(block.span);
            if block.expr.is_none();
            if let Some(last) = block.stmts.last();
            if let StmtKind::Semi(expr) = last.kind;
            let t_expr = cx.typeck_results().expr_ty(expr);
            if t_expr.is_unit();
            then {
                // make sure that the block does not belong to a function
                for (hir_id, _) in cx.tcx.hir().parent_iter(block.hir_id) {
                    if let Some(body_id) = cx.tcx.hir().maybe_body_owned_by(hir_id) {
                        if let BodyOwnerKind::Fn = cx.tcx.hir().body_owner_kind(hir_id) {
                            let item_body = cx.tcx.hir().body(body_id);
                            if let ExprKind::Block(fn_block, _) = item_body.value.kind {
                                if let Some(pot_if) = fn_block.expr {
                                    if let ExprKind::If(..) = pot_if.kind {
                                        return;
                                    }
                                }
                                if fn_block.hir_id == block.hir_id {
                                    return
                                }
                            }
                        }
                    }
                }
                // filter out other blocks and the desugared for loop
                if let ExprKind::Block(..) | ExprKind::DropTemps(..) = expr.kind { return }

                // make sure we're also having the semicolon at the end of the expression...
                let expr_w_sem = expand_span_to_semicolon(cx, expr.span);
                let expr_snip = snippet_with_macro_callsite(cx, expr_w_sem, "..");
                let mut expr_sugg = expr_snip.to_string();
                expr_sugg.pop();

                // and the block
                let block_w_sem = expand_span_to_semicolon(cx, block.span);
                let mut block_snip: String = snippet_with_macro_callsite(cx, block_w_sem, "..").to_string();
                if block_snip.ends_with('\n') {
                    block_snip.pop();
                }

                // retrieve the suggestion
                let suggestion = if block_snip.ends_with(';') {
                    block_snip.replace(expr_snip.as_ref(), &format!("{}", expr_sugg.as_str()))
                } else {
                    format!("{};", block_snip.replace(expr_snip.as_ref(), &format!("{}", expr_sugg.as_str())))
                };

                span_lint_and_sugg(
                    cx,
                    SEMICOLON_OUTSIDE_BLOCK,
                    if block_snip.ends_with(';') {
                        block_w_sem
                    } else {
                        block.span
                    },
                    "consider moving the `;` outside the block for consistent formatting",
                    "put the `;` outside the block",
                    suggestion,
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

/// Takes a span and extends it until after a semicolon in the last line of the span.
fn expand_span_to_semicolon<'tcx>(cx: &LateContext<'tcx>, expr_span: Span) -> Span {
    let expr_span_with_sem = cx.sess().source_map().span_extend_to_next_char(expr_span, ';', false);
    expr_span_with_sem.with_hi(expr_span_with_sem.hi().add(BytePos(1)))
}

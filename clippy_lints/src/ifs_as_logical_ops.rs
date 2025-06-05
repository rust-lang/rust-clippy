use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_never_expr;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::sym::clippy_utils;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.89.0"]
    pub IFS_AS_LOGICAL_OPS,
    nursery,
    "default lint description"
}
declare_lint_pass!(IfsAsLogicalOps => [IFS_AS_LOGICAL_OPS]);

impl<'tcx> LateLintPass<'tcx> for IfsAsLogicalOps {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        if let ExprKind::If(cond, cond_inner, Some(els)) = e.kind
            && let ExprKind::Block(if_block, _label) = cond_inner.kind
            // Check if the if-block has only a return statement
            && if_block.stmts.len() == 0
            && let Some(if_expr) = if_block.expr
            // and does not diverge.
            && is_never_expr(cx, if_expr).is_none()
            // And that the else block consists of only the boolean 'false'.
            && let ExprKind::Block(else_block, _label) = els.kind
            && else_block.stmts.len() == 0
            && let Some(else_expr) = else_block.expr
            && let ExprKind::Lit(lit) = else_expr.kind
            && let LitKind::Bool(value) = lit.node
            && value == false
        {
            let maybe_lhs_snippet = if_expr.span.get_source_text(cx);
            let maybe_rhs_snippet = lit.span.get_source_text(cx);
            if let Some(lhs_snippet) = maybe_lhs_snippet
                && let Some(rhs_snippet) = maybe_rhs_snippet
            {
                let lhs_text = lhs_snippet.as_str();
                let rhs_text = rhs_snippet.as_str();
                span_lint_and_sugg(
                    cx,
                    IFS_AS_LOGICAL_OPS,
                    e.span,
                    "Logical operations are clearer than if conditions in this instance",
                    "try",
                    format!("{lhs_text} && {rhs_text}"),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_ast::ast::*;
use rustc_ast::token::LitKind;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::kw;

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

impl EarlyLintPass for IfsAsLogicalOps {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, e: &Expr) {
        if let ExprKind::If(cond, cond_inner, Some(els)) = &e.kind
            && let Some(inner_if_stmt) = cond_inner.stmts.first()
            && let ExprKind::Block(inner_block, _label) = &els.kind
            && let Some(stmt) = inner_block.stmts.first()
            && let StmtKind::Expr(els_expr) = &stmt.kind
            && let ExprKind::Lit(lit) = &els_expr.kind
            && let LitKind::Bool = &lit.kind
            && lit.symbol == kw::False
        {
            let if_cond_snippet = snippet(cx, cond.span, "..");
            let conjunction_snippet = " && ";
            let inner_snippet = snippet(cx, inner_if_stmt.span, "..");
            let final_snippet = if_cond_snippet + conjunction_snippet + inner_snippet;

            span_lint_and_sugg(
                cx,
                IFS_AS_LOGICAL_OPS,
                e.span,
                "Logical operations are clearer than if conditions in this instance",
                "try",
                final_snippet.to_string(),
                Applicability::MachineApplicable,
            )
        }
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::edition::Edition;

declare_clippy_lint! {
    /// ### What it does
    /// Warns when a match, if let, or while let scrutinee is wrapped in a block.
    ///
    /// ### Why is this bad?
    /// Prior to the 2024 edition, wrapping the scrutinee in a block did not drop
    /// temporaries before the body executes.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if let Some(x) = { my_function() } { .. }
    /// ```
    #[clippy::version = "1.80.0"]
    pub BLOCK_SCRUTINEE,
    correctness,
    "warns when the scrutinee is wrapped in a block in older editions"
}

declare_lint_pass!(BlockScrutinee => [BLOCK_SCRUTINEE]);

impl<'tcx> LateLintPass<'tcx> for BlockScrutinee {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if cx.tcx.sess.edition() >= Edition::Edition2024 {
            return;
        }

        let scrutinee = match expr.kind {
            ExprKind::Match(scrutinee, _, _) => scrutinee,
            ExprKind::Let(let_expr) => let_expr.init,
            _ => return,
        };

        if let ExprKind::Block(block, _) = scrutinee.kind
            && block.stmts.is_empty()
            && let Some(inner_expr) = block.expr
        {
            let inner_snippet = snippet(cx, inner_expr.span, "..");

            span_lint_and_sugg(
                cx,
                BLOCK_SCRUTINEE,
                scrutinee.span,
                "scrutinee is wrapped in a block which will not drop temporaries until the end of the statement in this edition",
                "remove the block",
                inner_snippet.to_string(),
                Applicability::MachineApplicable,
            );
        }
    }
}

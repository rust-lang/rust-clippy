use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::edition::Edition;

declare_clippy_lint! {
    /// ### What it does
    /// Warns when a match, if let, or while let scrutinee is wrapped in a block.
    /// This lint only triggers on the 2021 edition and older.
    ///
    /// ### Why is this bad?
    /// It is unusual to write `{ expr }` when you could just have written
    /// `expr`, and it is unlikely that anyone would write that for any reason
    /// other than wanting temporaries in `expr` to be dropped before executing
    /// the body of the `match`/`if let`/`while` statement. However, prior to
    /// the 2024 edition, wrapping the scrutinee in a block did not drop
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

            span_lint_and_then(
                cx,
                BLOCK_SCRUTINEE,
                scrutinee.span,
                "temporary values in this block-wrapped scrutinee will not be dropped until the end of the statement",
                |diag| {
                    diag.note("this behavior is specific to Rust editions prior to 2024");
                    diag.note("in Rust 2024, temporaries in a block scrutinee drop immediately before the match arm or block body");
                    diag.help("if you want the temporaries to be dropped early, you can update your `Cargo.toml` to the 2024 edition");

                    diag.span_suggestion(
                        scrutinee.span,
                        "remove the block to yield the same behavior but with cleaner code",
                        inner_snippet.to_string(),
                        Applicability::MachineApplicable,
                    );

                    diag.span_suggestion(
                        scrutinee.span,
                        "if you intended to drop temporaries early, move the expression to a separate local binding",
                        format!("{{ let res = {inner_snippet}; res }}"),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }
}

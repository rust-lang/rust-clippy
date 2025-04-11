use clippy_utils::diagnostics::span_lint;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// let foo = "anything";
    /// if foo != "thing1" || foo != "thing2" {
    ///     println!("always executes");
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let foo = "anything";
    /// if foo != "thing1" && foo != "thing2" {
    ///     println!("sometimes executes");
    /// }
    /// ```
    #[clippy::version = "1.87.0"]
    pub ALWAYS_TRUE_CONDITIONS,
    nursery,
    "checks for if statement conditions which are always true"
}

declare_lint_pass!(AlwaysTrueConditions => [ALWAYS_TRUE_CONDITIONS]);

fn context_applicable(expr: &Expr<'_>) -> bool {
    if let ExprKind::Binary(new_op, new_f, new_l) = expr.kind {
        if new_op.node == BinOpKind::Or {
            //only continue DOWN if its an or.give me the zuck
            let f = context_applicable(new_f);
            let l = context_applicable(new_l);
            l && f
        } else {
            new_op.node == BinOpKind::Ne
        }
    } else {
        false
    }
}

impl LateLintPass<'_> for AlwaysTrueConditions {
    fn check_expr(&mut self, cx: &LateContext<'_>, e: &Expr<'_>) {
        if let ExprKind::If(cond, _, _) = e.kind
            && let ExprKind::DropTemps(cond) = cond.kind
            && let ExprKind::Binary(f_op_kind, f_cond, l_cond) = cond.kind
            && let BinOpKind::Or = f_op_kind.node
        {
            let msg = "expression will always be true, did you mean &&?";
            if context_applicable(f_cond) && context_applicable(l_cond) {
                span_lint(cx, ALWAYS_TRUE_CONDITIONS, e.span, msg);
            }
        }
    }
}

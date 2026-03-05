use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Flags a relativly common error where users comparing a varible to a primative use || instal of && in conjunction
    /// with !=. This lint was originally meant for simple n != 1 || n != 2 type statements, but the lint will detect
    /// the primitive and varible in any order for any length, as long as the variable stays the same, and the condition
    /// is always 1 primitive and 1 varible.
    ///
    /// ### Why is this bad?
    ///
    ///This is bad because the code will always result in true. If this is intentional a constant can be used in the
    ///case of a boolean varibale assignment, or code in an if block should just be moved outside with comments
    ///explaining why.
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

fn context_applicable(expr: &Expr<'_>) -> Option<Res> {
    if let ExprKind::Binary(new_op, new_f, new_l) = expr.kind {
        if new_op.node == BinOpKind::Or {
            let f = context_applicable(new_f);
            let l = context_applicable(new_l);
            if l == f { l } else { None }
        } else if new_op.node == BinOpKind::Ne {
            normalize_expression(new_f, new_l)
        } else {
            None
        }
    } else {
        None
    }
}

fn normalize_expression<'a>(f: &'a Expr<'a>, l: &'a Expr<'a>) -> Option<Res> {
    if let (ExprKind::Path(QPath::Resolved(_, path)), ExprKind::Lit(_)) = (f.kind, l.kind) {
        Some(path.res)
    } else if let (ExprKind::Lit(_), ExprKind::Path(QPath::Resolved(_, path))) = (f.kind, l.kind) {
        Some(path.res)
    } else {
        None
    }
}

impl LateLintPass<'_> for AlwaysTrueConditions {
    fn check_expr(&mut self, cx: &LateContext<'_>, e: &Expr<'_>) {
        if let ExprKind::Binary(f_op_kind, f_cond, l_cond) = e.kind
            && let BinOpKind::Or = f_op_kind.node
        {
            let msg = "expression will always be true, did you mean to use &&?";

            let f_res = context_applicable(f_cond);
            let l_res = context_applicable(l_cond);

            if f_res.is_some() && (l_res == f_res) {
                span_lint(cx, ALWAYS_TRUE_CONDITIONS, e.span, msg);
            }
        }
    }
}

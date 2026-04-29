use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LetExpr, LetStmt, Pat, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for patterns that don't bind any variables
    /// in `let-else`/`if-let`/`while-let` expressions.
    ///
    /// ### Why is this bad?
    /// This is unclear and confusing as it violates the intended purpose of `let`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let 0 = 1 else { return };
    /// if let Some(42) = opt {}
    /// while let Ok(None) = get_data() {}
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// if 0 != 1 { return }
    /// if !matches!(opt, Some(42)) {}
    /// while !matches!(get_data(), Ok(None)) {}
    /// ```
    #[clippy::version = "1.97.0"]
    pub NON_BINDING_LET_PATTERNS,
    restriction,
    "let pattern doesn't have bindings"
}

declare_lint_pass!(NonBindingLetPatterns => [NON_BINDING_LET_PATTERNS]);

impl LateLintPass<'_> for NonBindingLetPatterns {
    // check for `if-let` and `while-let`
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let ExprKind::Let(LetExpr { pat, init, .. }) = expr.kind else {
            return;
        };

        if !pat.walk_short(|p| !matches!(p.kind, PatKind::Binding(..))) {
            return;
        }

        let pat_snippet = snippet(cx, pat.span, "...");
        let init_snippet = snippet(cx, init.span, "...");
        let (help, sugg) = if can_compare(cx, pat) {
            let sugg = if matches!(pat.kind, PatKind::Struct(..)) {
                format!("{init_snippet} == ({pat_snippet})")
            } else {
                format!("{init_snippet} == {pat_snippet}")
            };
            ("consider equivalence check", sugg)
        } else {
            let sugg = format!("matches!({init_snippet}, {pat_snippet})");
            ("consider using `matches!`", sugg)
        };

        span_lint_and_sugg(
            cx,
            NON_BINDING_LET_PATTERNS,
            expr.span,
            "let pattern doesn't have bindings",
            help,
            sugg,
            Applicability::MachineApplicable,
        );
    }

    // check for `let-else`
    fn check_stmt(&mut self, cx: &LateContext<'_>, stmt: &Stmt<'_>) {
        let StmtKind::Let(LetStmt {
            pat,
            init: Some(init),
            els: Some(else_block),
            ..
        }) = stmt.kind
        else {
            return;
        };

        if !pat.walk_short(|p| !matches!(p.kind, PatKind::Binding(..))) {
            return;
        }

        let pat_snippet = snippet(cx, pat.span, "...");
        let init_snippet = snippet(cx, init.span, "...");
        let block_snippet = snippet(cx, else_block.span, "...");
        let (help, sugg) = if can_compare(cx, pat) {
            let sugg = if matches!(pat.kind, PatKind::Struct(..)) {
                format!("if {init_snippet} != ({pat_snippet}) {block_snippet}")
            } else {
                format!("if {init_snippet} != {pat_snippet} {block_snippet}")
            };
            ("consider equivalence check", sugg)
        } else {
            let sugg = format!("if !matches!({init_snippet}, {pat_snippet}) {block_snippet}");
            ("consider using `matches!`", sugg)
        };

        span_lint_and_sugg(
            cx,
            NON_BINDING_LET_PATTERNS,
            stmt.span,
            "let pattern doesn't have bindings",
            help,
            sugg,
            Applicability::MachineApplicable,
        );
    }
}

fn can_compare(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    let is_expr_like = pat.walk_short(|p| {
        matches!(
            p.kind,
            PatKind::Expr(..)
                | PatKind::Ref(..)
                | PatKind::Slice(..)
                | PatKind::Tuple(..)
                | PatKind::Struct(..)
                | PatKind::TupleStruct(..)
        )
    });

    let ty = cx.typeck_results().pat_ty(pat);
    let type_impl_partialeq = cx
        .tcx
        .lang_items()
        .eq_trait()
        .is_some_and(|id| implements_trait(cx, ty, id, &[ty.into()]));

    is_expr_like && type_impl_partialeq
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, LetStmt, Pat, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for patterns that don't bind any variables in `let-else` expressions.
    ///
    /// ### Why is this bad?
    /// This is unclear and confusing as it violates the intended purpose of `let`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let 0 = 1 else { return };
    /// let Some(42) = opt else { unreachable!() };
    /// let Ok(_) = result else { panic!() };
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// if 0 != 1 { return }
    /// if opt != Some(42) { unreachable!() }
    /// if !matches!(result, Ok(_)) { panic!() }
    /// ```
    #[clippy::version = "1.98.0"]
    pub NON_BINDING_LET_ELSE,
    pedantic,
    "let-else pattern doesn't have bindings"
}

declare_lint_pass!(NonBindingLetElse => [NON_BINDING_LET_ELSE]);

impl LateLintPass<'_> for NonBindingLetElse {
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

        let mut app = Applicability::MachineApplicable;
        let pat_snippet = snippet_with_applicability(cx, pat.span.source_callsite(), "...", &mut app);
        let init_snippet = snippet_with_applicability(cx, init.span.source_callsite(), "...", &mut app);
        let block_snippet = snippet_with_applicability(cx, else_block.span, "...", &mut app);

        if is_valid_for_eq(cx, pat, init) {
            let sugg = if matches!(pat.kind, PatKind::Struct(..)) {
                format!("if {init_snippet} != ({pat_snippet}) {block_snippet}")
            } else {
                format!("if {init_snippet} != {pat_snippet} {block_snippet}")
            };
            span_lint_and_sugg(
                cx,
                NON_BINDING_LET_ELSE,
                stmt.span.source_callsite(),
                "let pattern doesn't have bindings",
                "you may instead check for equivalence, but it may not behave as pattern matching",
                sugg,
                Applicability::MaybeIncorrect,
            );
        } else {
            let sugg = format!("if !matches!({init_snippet}, {pat_snippet}) {block_snippet}");
            span_lint_and_sugg(
                cx,
                NON_BINDING_LET_ELSE,
                stmt.span.source_callsite(),
                "let pattern doesn't have bindings",
                "consider using `matches!`",
                sugg,
                app,
            );
        }
    }
}

fn is_valid_for_eq(cx: &LateContext<'_>, pat: &Pat<'_>, init: &Expr<'_>) -> bool {
    let Some(partialeq_did) = cx.tcx.lang_items().eq_trait() else {
        return false;
    };

    let pat_ty = cx.typeck_results().pat_ty(pat);
    let init_ty = cx.typeck_results().expr_ty(init);
    if !implements_trait(cx, init_ty, partialeq_did, &[pat_ty.into()]) {
        return false;
    }

    pat.walk_short(|p| {
        matches!(
            p.kind,
            PatKind::Expr(..)
                | PatKind::Ref(..)
                | PatKind::Slice(..)
                | PatKind::Tuple(..)
                | PatKind::Struct(..)
                | PatKind::TupleStruct(..)
        )
    })
}

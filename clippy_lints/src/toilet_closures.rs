use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{Block, Closure, ClosureKind, Expr, ExprKind, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects a closure which takes one argument and returns unit.
    ///
    /// ### Why is this bad?
    ///
    /// This is a less clear way to write `drop`.
    ///
    /// ### Example
    /// ```no_run
    /// fn drop_ok<T, E>(result: Result<T, E>) -> Result<(), E> {
    ///     result.map(|_| ())
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn drop_ok<T, E>(result: Result<T, E>) -> Result<(), E> {
    ///     result.map(drop)
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub TOILET_CLOSURES,
    style,
    "checks for 'toilet closure' usage"
}

declare_lint_pass!(ToiletClosures => [TOILET_CLOSURES]);

fn is_empty_expr(expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Tup([])
            | ExprKind::Block(
                Block {
                    stmts: [],
                    expr: None,
                    ..
                },
                None,
            )
    )
}

impl<'tcx> LateLintPass<'tcx> for ToiletClosures {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        let ExprKind::Closure(Closure {
            fn_decl,
            body,
            kind: ClosureKind::Closure,
            ..
        }) = expr.kind
        else {
            return;
        };

        // Check that the closure takes one argument
        if let [arg_ty] = fn_decl.inputs
            // and returns `()` or `{}`
            && is_empty_expr(cx.tcx.hir().body(*body).value)
            // and the closure is not higher ranked, which `drop` cannot replace
            && let ty::Closure(_, generic_args) = cx.typeck_results().expr_ty(expr).kind()
            && generic_args.as_closure().sig().bound_vars().is_empty()
        {
            let mut applicability = Applicability::MachineApplicable;
            span_lint_and_sugg(
                cx,
                TOILET_CLOSURES,
                expr.span,
                "defined a 'toilet closure'",
                "replace with",
                if matches!(arg_ty.kind, TyKind::Infer) {
                    String::from("drop")
                } else {
                    let arg_ty_snippet = snippet_with_applicability(cx, arg_ty.span, "...", &mut applicability);
                    format!("drop::<{arg_ty_snippet}>",)
                },
                applicability,
            );
        }
    }
}

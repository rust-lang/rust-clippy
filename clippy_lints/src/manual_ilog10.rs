use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions like `x.ilog(10)`, which is a manual reimplementation of
    /// `x.ilog10()`.
    ///
    /// ### Why is this bad?
    /// `ilog10()` is more performant than `ilog(10)`.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let nb_digits = x.ilog(10);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// let x: u32 = 5;
    /// let nb_digits = x.ilog10();
    /// ```
    #[clippy::version = "1.98.0"]
    pub MANUAL_ILOG10,
    perf,
    "using `ilog(10)` instead of `ilog10()`"
}

declare_lint_pass!(ManualIlog10 => [MANUAL_ILOG10]);

impl LateLintPass<'_> for ManualIlog10 {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        if let ExprKind::MethodCall(ilog, recv, [base], _) = expr.kind
            && expr.span.eq_ctxt(base.span)
            && ilog.ident.name == sym::ilog
            && let ExprKind::Lit(lit) = base.kind
            && let LitKind::Int(Pu128(10), _) = lit.node
            && cx.typeck_results().expr_ty_adjusted(recv).is_integral()
            /* no need to check MSRV here, as `ilog` and `ilog10` were introduced simultaneously */
            && !is_from_proc_macro(cx, expr)
        {
            span_lint_and_sugg(
                cx,
                MANUAL_ILOG10,
                ilog.ident.span.with_hi(expr.span.hi()),
                "manually reimplementing `ilog10`",
                "try",
                "ilog10()".to_owned(),
                Applicability::MachineApplicable,
            );
        }
    }
}

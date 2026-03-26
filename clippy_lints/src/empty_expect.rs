use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef;
use clippy_utils::sym;
use rustc_ast::ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects `.expect("")` calls where the expect message is an empty string literal.
    ///
    /// ### Why is this bad?
    /// An empty expect message provides no more context than a plain `.unwrap()` call.
    /// It is sometimes used to bypass the `unwrap_used` lint without actually providing
    /// a useful panic message.
    ///
    /// ### Example
    /// ```no_run
    /// let x: Option<i32> = Some(1);
    /// let _ = x.expect("");
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: Option<i32> = Some(1);
    /// let _ = x.expect("x should always be Some because ...");
    /// ```
    #[clippy::version = "1.96.0"]
    pub EMPTY_EXPECT,
    suspicious,
    "`.expect(\"\")` with an empty string provides no useful panic message"
}
declare_lint_pass!(EmptyExpect => [EMPTY_EXPECT]);

impl<'tcx> LateLintPass<'tcx> for EmptyExpect {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::MethodCall(method, recv, [arg], _) = expr.kind
            && method.ident.name == sym::expect
            && is_empty_str_lit(arg)
        {
            let ty = cx.typeck_results().expr_ty(recv).peel_refs();
            if matches!(ty.opt_diag_name(cx), Some(sym::Option | sym::Result)) {
                span_lint_and_help(
                    cx,
                    EMPTY_EXPECT,
                    expr.span,
                    "`.expect(\"\")` with an empty string provides no useful panic message",
                    None,
                    "provide a meaningful expect message or use `.unwrap()` instead",
                );
            }
        }
    }
}

fn is_empty_str_lit(expr: &Expr<'_>) -> bool {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Str(sym, _) = lit.node
    {
        sym.is_empty()
    } else {
        false
    }
}

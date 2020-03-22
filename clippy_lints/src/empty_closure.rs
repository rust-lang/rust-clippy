use crate::utils::{match_def_path, paths, span_lint};
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Checks for empty closure.
    ///
    /// **Why is this bad?** Empty closure does nothing, it can be removed.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// std::thread::spawn(|| {});
    /// ```
    pub EMPTY_CLOSURE,
    style,
    "closure with empty body"
}

declare_lint_pass!(EmptyClosure => [EMPTY_CLOSURE]);

impl LateLintPass<'_, '_> for EmptyClosure {
    fn check_expr(&mut self, cx: &LateContext<'_, '_>, expr: &'_ Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(ref path_expr, ref args) = expr.kind;
            if args.len() == 1;
            if let ExprKind::Path(ref qpath) = path_expr.kind;
            if let Some(def_id) = cx.tables.qpath_res(qpath, path_expr.hir_id).opt_def_id();
            if match_def_path(cx, def_id, &paths::THREAD_SPAWN);
            if let ExprKind::Closure(_, _, body_id, ..) = args[0].kind;
            let body = cx.tcx.hir().body(body_id);
            if let ExprKind::Block(ref b, _) = body.value.kind;
            if b.stmts.is_empty() && b.expr.is_none();
            then {
                span_lint(
                    cx,
                    EMPTY_CLOSURE,
                    args[0].span,
                    "Empty closure may be removed",
                );
            }
        }
    }
}

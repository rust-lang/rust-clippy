use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    /// Too verbose
    ///
    /// ### Example
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write(Path::new("foo.txt"), "foo");
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write("foo.txt", "foo");
    /// ```
    #[clippy::version = "1.88.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "default lint description"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if check(cx, expr) {
            span_lint_and_help(
                cx,
                NEEDLESS_PATH_NEW,
                expr.span,
                "`Path::new` used",
                None,
                "consider removing `Path::new`",
            );
        }
    }
}

fn check(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::MethodCall(method_name, receiver, args, _) = expr.kind
        && method_name.ident.as_str() == "unwrap"
        && let ExprKind::Call(func, args1) = receiver.kind
        && let ExprKind::Path(ref qpath) = func.kind
        // && match_qpath(qpath, &["fs", "copy"])
        && args1.len() == 2
        && let ExprKind::Call(func1, args2) = args1[0].kind
        && let ExprKind::Path(ref qpath1) = func1.kind
        && match_qpath(qpath1, &["path", "Path", "new"])
        && args2.len() == 1
        && let ExprKind::Lit(ref lit) = args2[0].kind
        && let LitKind::Str(s, _) = lit.node
        && s.as_str() == "foo"
        && let ExprKind::Lit(ref lit1) = args1[1].kind
        && let LitKind::Str(s1, _) = lit1.node
        // && s1.as_str() == "a"
        && args.is_empty()
    {
        true
    } else {
        false
    }
}

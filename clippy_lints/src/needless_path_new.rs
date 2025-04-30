use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects expressions being enclosed in `Path::new` when passed to a function that accepts
    /// `impl AsRef<Path>`, when the enclosed expression could be used.
    ///
    /// ### Why is this bad?
    /// It is unnecessarily verbose
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
    #[clippy::version = "1.90.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "an argument passed to a function  that accepts `impl AsRef<Path>` \
    being enclosed in `Path::new` when the argument implements the trait"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
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

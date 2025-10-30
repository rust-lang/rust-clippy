use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::paths::{PathNS, lookup_path_str};
use clippy_utils::{fn_def_id, is_in_test_function};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for use of `std::env::set_env` in tests.
    ///
    /// ### Why restrict this?
    /// Setting environment varibales in tests often means the subject code
    /// is reading and acting on the environment. By default, rust tests
    /// are run concurrently, and setting environment variables cannot be
    /// done in a way that is scoped only to the test. Even if care is taken
    /// to clean up any mutations, concurrent test runs will affect each
    /// other's environment.
    ///
    /// ### Example
    /// ```no_run
    /// #[cfg(test)]
    /// mod tests {
    ///     fn my_test() {
    ///         unsafe std::env::set_var("MY_VAR", "1");
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.92.0"]
    pub SET_ENV_IN_TESTS,
    restriction,
    "use of set_env in tests"
}
declare_lint_pass!(SetEnvInTests => [SET_ENV_IN_TESTS]);

impl<'tcx> LateLintPass<'tcx> for SetEnvInTests {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let set_var_ids = lookup_path_str(cx.tcx, PathNS::Value, "std::env::set_var");

        if matches!(expr.kind, ExprKind::Call(..))
            && let Some(call_def_id) = fn_def_id(cx, expr)
            && is_in_test_function(cx.tcx, expr.hir_id)
            && set_var_ids.contains(&call_def_id)
        {
            span_lint_and_help(
                cx,
                SET_ENV_IN_TESTS,
                expr.span,
                "env::set_var called from a test",
                None,
                "this might indicate state leakage and cause flaky tests",
            );
        }
    }
}

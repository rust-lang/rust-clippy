use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why restrict this?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.92.0"]
    pub SET_ENV_IN_TESTS,
    restriction,
    "default lint description"
}
declare_lint_pass!(SetEnvInTests => [SET_ENV_IN_TESTS]);

impl LateLintPass<'_> for SetEnvInTests {}

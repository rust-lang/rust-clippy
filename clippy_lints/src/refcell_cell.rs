use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.99.0"]
    pub REFCELL_CELL,
    perf,
    "default lint description"
}
declare_lint_pass!(RefcellCell => [REFCELL_CELL]);

impl LateLintPass<'_> for RefcellCell {}

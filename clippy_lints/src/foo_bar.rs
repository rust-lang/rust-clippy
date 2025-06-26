use rustc_lint::LateLintPass;
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
    #[clippy::version = "1.89.0"]
    pub FOO_BAR,
    nursery,
    "default lint description"
}
declare_lint_pass!(FooBar => [FOO_BAR]);

impl LateLintPass<'_> for FooBar {}

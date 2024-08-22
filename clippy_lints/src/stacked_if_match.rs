use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for stacked `if` and `match`, e.g., `if if`.
    ///
    /// ### Why is this bad?
    /// Stacked `if`'s and `match`'s are hard to read.
    ///
    /// ### Example
    /// ```no_run
    /// if if a == b {
    ///     c == d
    /// } else {
    ///     e == f
    /// } {
    ///     println!("true");
    /// } else {
    ///     println!("false");
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let cond = if a == b {
    ///     c == d
    /// } else {
    ///     e == f
    /// };
    ///
    /// if cond {
    ///     println!("true");
    /// } else {
    ///     println!("false");
    /// }
    /// ```
    #[clippy::version = "1.82.0"]
    pub STACKED_IF_MATCH,
    style,
    "`if if` and `match match` that can be eliminated"
}

declare_lint_pass!(StackedIfMatch => [STACKED_IF_MATCH]);

impl<'tcx> LateLintPass<'tcx> for StackedIfMatch {
   fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {

    }
}

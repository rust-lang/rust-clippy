use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::LetStmt;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for declared variables with undeclared types.
    ///
    /// ### Why restrict this?
    /// Relying on type inference makes code much less readable outside of an IDE where type inlays can be inserted.
    /// Restricting this feature of the language makes ambiguous code easier to read in plain-text such as when collaborating with others
    /// using version control software and hosting platforms for code review.
    ///
    /// ### Example
    /// ```no_run
    /// // A function from anywhere in the codebase that might not be visible
    /// fn ambiguous_function() -> Vec<i32> {vec![]}
    ///
    /// let variable = ambiguous_function();
    /// ```
    /// Use instead:
    /// ```no_run
    /// // Same as above, but now the return type doesn't have to be inferred from context
    /// fn ambiguous_function() -> Vec<i32> {vec![]}
    ///
    /// let variable : Vec<i32> = ambiguous_function;
    /// ```
    #[clippy::version = "1.101.0"]
    pub UNDECLARED_TYPE,
    restriction,
    "declared variable with an undeclared type"
}

declare_lint_pass!(UndeclaredType => [UNDECLARED_TYPE]);

impl LateLintPass<'_> for UndeclaredType {
    fn check_local(&mut self, cx: &LateContext<'_>, local: &LetStmt<'_>) {
        if local.span.from_expansion() {
            return;
        }
        if local.ty.is_some() {
            return;
        }
        span_lint_and_help(
            cx,
            UNDECLARED_TYPE,
            local.span,
            "declared variable has an undeclared type",
            Some(local.span),
            "declare a type for this variable",
        );
    }
}

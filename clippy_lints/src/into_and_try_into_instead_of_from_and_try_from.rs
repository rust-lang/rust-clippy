use rustc_lint::{LateLintPass, LateContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_hir::*;

declare_clippy_lint! {
    /// **What it does:**
    ///
    /// **Why is this bad?**
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    pub INTO_AND_TRY_INTO_INSTEAD_OF_FROM_AND_TRY_FROM,
    style,
    "default lint description"
}

declare_lint_pass!(IntoAndTryIntoInsteadOfFromAndTryFrom => [INTO_AND_TRY_INTO_INSTEAD_OF_FROM_AND_TRY_FROM]);

impl LateLintPass<'_> for IntoAndTryIntoInsteadOfFromAndTryFrom {}

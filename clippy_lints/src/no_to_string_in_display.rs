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
    pub NO_TO_STRING_IN_DISPLAY,
    correctness,
    "default lint description"
}

declare_lint_pass!(NoToStringInDisplay => [NO_TO_STRING_IN_DISPLAY]);

impl LateLintPass<'_, '_> for NoToStringInDisplay {}

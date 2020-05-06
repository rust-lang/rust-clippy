use rustc_lint::{LateLintPass, LateContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_hir::*;

declare_clippy_lint! {
    /// **What it does:** Checks for uses of `to_string()` when implementing
    /// `Display` traits.
    ///
    /// **Why is this bad?** Usually `to_string` is implemented indirectly
    /// via `Display`. Hence using it while implementing `Display` would
    /// lead to infinite recursion.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// impl fmt::Display for Structure {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{}", self.to_string())
    ///     }
    /// }
    ///
    /// ```
    /// Use instead:
    /// ```rust
    /// impl fmt::Display for Structure {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{}", self.0)
    ///     }
    /// }
    /// ```
    pub NO_TO_STRING_IN_DISPLAY,
    correctness,
    "to_string method used while implementing Display trait"
}

declare_lint_pass!(NoToStringInDisplay => [NO_TO_STRING_IN_DISPLAY]);

impl LateLintPass<'_, '_> for NoToStringInDisplay {}

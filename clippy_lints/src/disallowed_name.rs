use crate::utils::span_lint;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{Pat, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// **What it does:** Checks for usage of disallowed names for variables, such
    /// as `foo`.
    ///
    /// **Why is this bad?** These names are usually placeholder names and should be
    /// avoided.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// let foo = 3.14;
    /// ```
    pub DISALLOWED_NAME,
    style,
    "usage of a disallowed/placeholder name"
}

#[derive(Clone, Debug)]
pub struct DisallowedName {
    disallowlist: FxHashSet<String>,
}

impl DisallowedName {
    pub fn new(disallowlist: FxHashSet<String>) -> Self {
        Self { disallowlist }
    }
}

impl_lint_pass!(DisallowedName => [DISALLOWED_NAME]);

impl<'tcx> LateLintPass<'tcx> for DisallowedName {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'_>) {
        if let PatKind::Binding(.., ident, _) = pat.kind {
            if self.disallowlist.contains(&ident.name.to_string()) {
                span_lint(
                    cx,
                    DISALLOWED_NAME,
                    ident.span,
                    &format!("use of a disallowed/placeholder name `{}`", ident.name),
                );
            }
        }
    }
}

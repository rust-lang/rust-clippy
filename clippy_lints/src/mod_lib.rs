use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.66.0"]
    pub MOD_LIB,
    pedantic,
    "default lint description"
}

#[derive(Default)]
pub struct ModLib;

impl_lint_pass!(ModLib => [MOD_LIB]);

impl<'tcx> LateLintPass<'tcx> for ModLib {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Mod(_) = item.kind {
            if item.ident.name == sym::lib {
                span_lint_and_help(
                    cx,
                    MOD_LIB,
                    item.span,
                    "uncommon use of mod::lib",
                    None,
                    "you probably meant use::package instead",
                );
            }
        }
    }
}

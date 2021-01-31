// use crate::utils::span_lint_and_help;
use crate::utils::span_lint;
use rustc_lint::{LateLintPass, LateContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_hir::*;
// use rustc_span::Span;
use rustc_span::symbol::sym;
use if_chain::if_chain;

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
    pub INTO_INSTEAD_OF_FROM,
    style,
    "default lint description"
}

declare_lint_pass!(IntoInsteadOfFrom => [INTO_INSTEAD_OF_FROM]);

impl LateLintPass<'tcx> for IntoInsteadOfFrom {
    fn check_where_predicate(&mut self, cx: &LateContext<'tcx>, wp: &'tcx WherePredicate<'tcx>) {
        match wp {
            WherePredicate::BoundPredicate(wbp) => {
                if_chain! {
                    if let Some(tr_ref) = wbp.bounds[0].trait_ref();
                    if let Some(def_id) = tr_ref.trait_def_id();
                    if cx.tcx.is_diagnostic_item(sym::from_trait, def_id);
                    then {
                        if cx.tcx.is_diagnostic_item(sym::from_trait, def_id) {
                            span_lint(
                                cx,
                                INTO_INSTEAD_OF_FROM,
                                wp.span(),
                                "That is from_trait"
                            );
                        };
                        if cx.tcx.is_diagnostic_item(sym::try_from_trait, def_id) {
                            span_lint(
                                cx,
                                INTO_INSTEAD_OF_FROM,
                                wp.span(),
                                "That is try_from_trait"
                            );
                        };
                    }
                }
            },
            _ => (),
        };
        
    }
}

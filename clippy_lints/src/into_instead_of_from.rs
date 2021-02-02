// use crate::utils::span_lint_and_help;
use crate::utils::span_lint_and_sugg;
use rustc_lint::{LateLintPass, LateContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_hir::*;
// use rustc_span::Span;
use rustc_span::symbol::sym;
use if_chain::if_chain;

declare_clippy_lint! {
    /// **What it does:** Checking for using of From or TryFrom trait as a generic bound.
    ///
    /// **Why is this bad?** Into and TryInto are supersets of From and TryFrom. Due to 
    /// coherence rules, sometimes From and TryFrom are forbid to implemented but Into and 
    /// TryInto are not. So Into is a more generic bound than From, We should choose Into or
    /// TryInto instead of From or TryFrom.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// fn foo<T>(a: T) where u32: From<T> {}
    /// fn bar<T>(a: T) where u32: TryFrom<T> {}
    /// ```
    /// Use instead:
    /// ```rust
    /// fn foo<T>(a: T) where T: Into<u32> {}
    /// fn bar<T>(a: T) where T: TryInto<u32> {}
    /// ```
    pub INTO_INSTEAD_OF_FROM,
    style,
    "Into or TryInto trait is a better choice than From or TryFrom trait as a generic bound"
}

declare_lint_pass!(IntoInsteadOfFrom => [INTO_INSTEAD_OF_FROM]);

impl LateLintPass<'tcx> for IntoInsteadOfFrom {
    fn check_where_predicate(&mut self, cx: &LateContext<'tcx>, wp: &'tcx WherePredicate<'tcx>) {
        match wp {
            WherePredicate::BoundPredicate(wbp) => {
                if_chain! {
                    if let Some(tr_ref) = wbp.bounds[0].trait_ref();
                    if let Some(def_id) = tr_ref.trait_def_id();
                    then {
                        if cx.tcx.is_diagnostic_item(sym::from_trait, def_id) {
                            let sugg = ""; //Todo
                            span_lint_and_sugg(
                                cx,
                                INTO_INSTEAD_OF_FROM,
                                wp.span(),
                                "Into trait is a more preferable choice than From as a generic bound",
                                format!("try `{}` instead", sugg),
                                sugg,
                                Applicability::MachineApplicable
                            );
                        };
                        if cx.tcx.is_diagnostic_item(sym::try_from_trait, def_id) {
                            let sugg = ""; //Todo
                            span_lint_and_sugg(
                                cx,
                                INTO_INSTEAD_OF_FROM,
                                wp.span(),
                                "TryInto trait is a more preferable choice than TryFrom as a generic bound",
                                format!("try `{}` instead", sugg),
                                sugg,
                                Applicability::MachineApplicable
                            );
                        };
                    }
                }
            },
            _ => (),
        };
        
    }
}

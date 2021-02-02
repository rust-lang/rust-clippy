// use crate::utils::span_lint_and_help;
use crate::utils::span_lint;
use rustc_lint::{LateLintPass, LateContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_hir::*;
// use rustc_span::Span;
use rustc_span::symbol::sym;
use if_chain::if_chain;

declare_clippy_lint! {
    /// **What it does:** Checking for using of Into or TryInto trait as a generic bound.
    ///
    /// **Why is this bad?** Into and TryInto are supersets of From and TryFrom. Due to 
    /// coherence rules, sometimes Into and TryInto are forbid to implemented but From and 
    /// TryFrom are not. So Into is a more generic bound than From, We should choose Into or
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
    /// fn bar<T>(a: T) where T: TryFrom<u32> {}
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

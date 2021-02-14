use crate::utils::snippet;
use crate::utils::span_lint_and_sugg;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

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
    pub FROM_INSTEAD_OF_INTO,
    style,
    "Into or TryInto trait is a better choice than From or TryFrom trait as a generic bound"
}

declare_lint_pass!(FromInsteadOfInto => [FROM_INSTEAD_OF_INTO]);

impl LateLintPass<'tcx> for FromInsteadOfInto {
    fn check_where_predicate(&mut self, cx: &LateContext<'tcx>, wp: &'tcx WherePredicate<'tcx>) {
        match wp {
            WherePredicate::BoundPredicate(wbp) => {
                if_chain! {
                    if let Some(tr_ref) = wbp.bounds[0].trait_ref();
                    if let Some(def_id) = tr_ref.trait_def_id();
                    if let Some(last_seg) = tr_ref.path.segments.last();
                    if let Some(generic_arg) = last_seg.args().args.first();
                    then {
                        let bounded_ty = snippet(cx, wbp.bounded_ty.span, "..");
                        let generic_arg_of_from_or_try_from = snippet(cx, generic_arg.span(), "..");

                        if cx.tcx.is_diagnostic_item(sym::from_trait, def_id) {
                            let sugg = format!("{}: Into<{}>", generic_arg_of_from_or_try_from, bounded_ty);
                            span_lint_and_sugg(
                                cx,
                                FROM_INSTEAD_OF_INTO,
                                wp.span(),
                                "Into trait is preferable than From as a generic bound",
                                "try",
                                sugg,
                                Applicability::MachineApplicable
                            );
                        };

                        if cx.tcx.is_diagnostic_item(sym::try_from_trait, def_id) {
                            let sugg = format!("{}: TryInto<{}>", generic_arg_of_from_or_try_from, bounded_ty);
                            span_lint_and_sugg(
                                cx,
                                FROM_INSTEAD_OF_INTO,
                                wp.span(),
                                "TryInto trait is preferable than TryFrom as a generic bound",
                                "try",
                                sugg,
                                Applicability::MaybeIncorrect
                            );
                        };
                    }
                }
            },
            _ => (),
        };
    }
}

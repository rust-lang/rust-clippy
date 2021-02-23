use crate::utils::span_lint_and_then;
use crate::utils::snippet_opt;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{GenericBound, WherePredicate};
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
        fn is_target_generic_bound(cx: &LateContext<'tcx>, b: &GenericBound<'_>) -> bool {
            if_chain! {
                if let Some(r) = b.trait_ref();
                if let Some(def_id) = r.trait_def_id();
                then {
                    cx.tcx.is_diagnostic_item(sym::from_trait, def_id) ||
                    cx.tcx.is_diagnostic_item(sym::try_from_trait, def_id)
                } else {
                    false
                }
            }
        }

        if let WherePredicate::BoundPredicate(wbp) = wp {
            if_chain! {
                let bounds = wbp.bounds;
                if let Some(position) = bounds.iter().position(|b| is_target_generic_bound(cx, b));
                let target_bound = &bounds[position];
                if let Some(tr_ref) = target_bound.trait_ref();
                if let Some(def_id) = tr_ref.trait_def_id();
                if let Some(last_seg) = tr_ref.path.segments.last();
                if let Some(generic_arg) = last_seg.args().args.first();
                if let Some(bounded_ty) = snippet_opt(cx, wbp.bounded_ty.span);
                if let Some(generic_arg_of_from_or_try_from) = snippet_opt(cx, generic_arg.span());
                then {
                    let replace_trait_name;
                    let target_trait_name;
                    if cx.tcx.is_diagnostic_item(sym::from_trait, def_id) {
                        replace_trait_name = "Into";
                        target_trait_name = "From";
                    } else if cx.tcx.is_diagnostic_item(sym::try_from_trait, def_id) {
                        replace_trait_name = "TryInto";
                        target_trait_name = "TryFrom";
                    } else {
                        return;
                    }
                    let message = format!("{} trait is preferable than {} as a generic bound", replace_trait_name, target_trait_name);
                    let switched_predicate = format!("{}: {}<{}>", generic_arg_of_from_or_try_from, replace_trait_name, bounded_ty);

                    let low;
                    if position == 0 {
                        low = wp.span().lo();
                    } else {
                        let previous_bound = &bounds[position -1];
                        low = previous_bound.span().hi();
                    }
                    let removed_span = target_bound.span().with_lo(low);

                    span_lint_and_then(
                        cx,
                        FROM_INSTEAD_OF_INTO,
                        wp.span(),
                        &message,
                        |diag| {
                            diag.span_suggestion(
                                removed_span,
                                &format!("remove {} bound", target_trait_name),
                                "".to_string(),
                                Applicability::MaybeIncorrect,
                            );

                            let sugg;
                            if bounds.len() == 1 {
                                sugg = switched_predicate;
                            } else {
                                sugg = format!(", {}", switched_predicate);
                            }
                            diag.span_suggestion(
                                wp.span().with_lo(wp.span().hi()),
                                "Add this bound predicate",
                                sugg,
                                Applicability::MaybeIncorrect,
                            );
                        }
                    );
                }
            }
        }
    }
}

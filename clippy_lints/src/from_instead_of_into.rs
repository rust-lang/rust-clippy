use crate::utils::snippet;
use crate::utils::span_lint_and_sugg;
use std::ptr;
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
        let is_target_generic_bound = |b| {
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
        };
        match wp {
            WherePredicate::BoundPredicate(wbp) => {
                if_chain! {
                    if let Some(target_bound) = wbp.bounds.iter().find(|b| is_target_generic_bound(b));
                    if let Some(tr_ref) = target_bound.trait_ref();
                    if let Some(def_id) = tr_ref.trait_def_id();
                    if let Some(last_seg) = tr_ref.path.segments.last();
                    if let Some(generic_arg) = last_seg.args().args.first();
                    if let Some(bounded_ty) = snippet_opt(cx, wbp.bounded_ty.span);
                    if let Some(generic_arg_of_from_or_try_from) = snippet_opt(cx, generic_arg.span());
                    if wbp.bounds.len() == 1;
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
                            replace_trait_name = "";
                            target_trait_name = "";
                        }

                        if !replace_trait_name.is_empty() && !target_trait_name.is_empty() {
                            let message = format!("{} trait is preferable than {} as a generic bound", replace_trait_name, target_trait_name);
                            let extracted_where_predicate = format!("{}: {}<{}>,", generic_arg_of_from_or_try_from, replace_trait_name, bounded_ty);
                            let sugg;
                            if wbp.bounds.len() == 1 {
                                sugg = extracted_where_predicate;
                            } else {
                                let bounds: &[&GenericBound<'_>] = wbp.bounds.iter().filter(|b| !ptr::eq(*b, target_bound)).collect();
                                let bounds = bounds.map(|b| snippet(cx, b.span(), ".."));
                                let first = bounds[0].to_string();
                                let bounds_str = bounds[1..].fold(first, |s, &e| s + " + " + snippet(cx, e.span(), ".."));
                                sugg = format!("{} {}: {}", extracted_where_predicate, bounded_ty, bounds_str);
                            }
                            span_lint_and_sugg(
                                cx,
                                FROM_INSTEAD_OF_INTO,
                                wp.span(),
                                &message,
                                "try",
                                sugg,
                                Applicability::MaybeIncorrect
                            );
                        }
                    }
                }
            },
            _ => (),
        };
    }
}

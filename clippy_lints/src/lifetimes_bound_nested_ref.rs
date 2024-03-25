/// Lints to help dealing with unsoundness due to a compiler bug described here:
/// <https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md>.
///
/// For the following three cases the current compiler (1.76.0) gives a later error message when
/// manually adding a generic lifetime bound that is implied by a nested reference:
///
///     Issue 25860:
///     Implied bounds on nested references + variance = soundness hole
///     
///     Issue 84591:
///     HRTB on subtrait unsoundly provides HTRB on supertrait with weaker implied bounds
///     
///     Issue 100051:
///     implied bounds from projections in impl header can be unsound
///     
/// The lints here suggest to manually add such lifetime bounds in the hope that
/// the unsoundness is avoided.
///
/// There are also reverse lints that suggest to remove lifetime bounds
/// that are implied by nested references. These lints are intended to be used only
/// after the compiler handles these lifetime bounds correctly.
///
/// All lints here are in the nursery category.
use std::cmp::Ordering;
use std::collections::BTreeSet;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl, GenericArg, GenericBound, Generics, Item, ItemKind, ParamName, WherePredicate};
use rustc_hir_analysis::hir_ty_to_ty;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::ty_kind::TyKind;
use rustc_middle::ty::{Region, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol};

extern crate rustc_type_ir;
use rustc_type_ir::AliasKind;

extern crate rustc_hash;
use rustc_hash::FxHashSet;

declare_clippy_lint! {
    /// ### What it does
    /// Checks function arguments and return values that have a nested reference type with lifetimes,
    /// and suggests to add the implied generic lifetime bounds.
    /// Adding a lifetimes bound helps to avoid unsound code because this addition
    /// can lead to a compiler error in related source code, as observed in rustc 1.76.0.
    ///
    /// ### Why is this bad?
    /// This is described in issue 25860,
    /// and as one case of unsoundness here:
    /// <https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md>.
    ///
    /// ### Known problems
    /// It is not known whether this covers all cases in issue 25860.
    ///
    /// ### Example, the `val_a` argument implies a lifetimes bound:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b: 'a, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```
    #[clippy::version = "1.78.0"]
    pub IMPLICIT_LIFETIMES_BOUND_NESTED_REF,
    nursery,
    "suggest to add generic lifetime bounds implied by nested references in function arguments and return value"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks function arguments and return values that have a nested reference type with lifetimes,
    /// and suggests to remove generic lifetime bounds that are implied.
    ///
    /// ### Why is this bad?
    /// Such generic lifetime bounds are redundant.
    ///
    /// ### Known problems
    /// Removing redundant lifetime bounds should only be done after the compiler
    /// has been fixed to deal correctly with implied lifetime bounds.
    ///
    /// ### Example, the `val_a` argument implies a lifetimes bound:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b: 'a, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```

    #[clippy::version = "1.78.0"]
    pub EXPLICIT_LIFETIMES_BOUND_NESTED_REF,
    nursery,
    "suggest to remove generic lifetime bounds implied by nested references in function arguments and return value"
}

pub struct LifetimesBoundNestedRef;

impl_lint_pass!(LifetimesBoundNestedRef => [
    IMPLICIT_LIFETIMES_BOUND_NESTED_REF,
    EXPLICIT_LIFETIMES_BOUND_NESTED_REF,
]);

impl<'tcx> LateLintPass<'tcx> for LifetimesBoundNestedRef {
    /// For issue 25860
    fn check_fn<'tcx2>(
        &mut self,
        cx: &LateContext<'tcx2>,
        fn_kind: FnKind<'tcx2>,
        _fn_decl: &'tcx2 FnDecl<'tcx2>,
        _body: &'tcx2 Body<'tcx2>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        let FnKind::ItemFn(_ident, generics, _fn_header) = fn_kind else {
            return;
        };
        let declared_lifetimes = get_declared_lifetimes(generics);
        if declared_lifetimes.len() <= 1 {
            return;
        }
        let mut linter = ImpliedBoundsLinter::new(declared_lifetimes, generics);
        // collect bounds implied by nested references in input types and output type
        let fn_sig = cx.tcx.fn_sig(local_def_id).skip_binder().skip_binder();
        for input_ty in fn_sig.inputs() {
            linter.collect_implied_lifetimes_bounds(*input_ty);
        }
        linter.collect_implied_lifetimes_bounds(fn_sig.output());
        linter.report_lints(cx);
    }

    /// For issues 84591 and 100051
    fn check_item_post<'tcx2>(&mut self, cx: &LateContext<'tcx2>, item: &'tcx2 Item<'tcx2>) {
        let ItemKind::Impl(impl_item) = item.kind else {
            return;
        };
        let Some(of_trait_ref) = impl_item.of_trait else {
            return;
        };
        let declared_lifetimes = get_declared_lifetimes(impl_item.generics);
        if declared_lifetimes.len() <= 1 {
            return;
        }
        let mut linter = ImpliedBoundsLinter::new(declared_lifetimes, impl_item.generics);
        for path_segment in of_trait_ref.path.segments {
            if let Some(generic_args) = path_segment.args {
                for generic_arg in generic_args.args {
                    if let GenericArg::Type(hir_arg_ty) = generic_arg {
                        let arg_ty = hir_ty_to_ty(cx.tcx, hir_arg_ty);
                        linter.collect_implied_lifetimes_bounds(arg_ty);
                    }
                }
            }
        }
        // issue 10051 for clause: impl ... for for_clause_ty
        let for_clause_ty = hir_ty_to_ty(cx.tcx, impl_item.self_ty);
        linter.collect_implied_lifetimes_bounds(for_clause_ty);
        linter.report_lints(cx);
    }
}

#[derive(Debug)]
struct BoundLftPair {
    long_lft: String,
    outlived_lft: String,
}

impl BoundLftPair {
    fn new(long_lft_sym: Symbol, outlived_lft_sym: Symbol) -> Self {
        BoundLftPair {
            long_lft: long_lft_sym.to_ident_string(),
            outlived_lft: outlived_lft_sym.to_ident_string(),
        }
    }

    fn as_bound_declaration(&self) -> String {
        format!("{}: {}", self.long_lft, self.outlived_lft,)
    }
}

impl PartialEq for BoundLftPair {
    fn eq(&self, other: &Self) -> bool {
        self.long_lft.eq(&other.long_lft) && self.outlived_lft.eq(&other.outlived_lft)
    }
}

impl Eq for BoundLftPair {}

impl PartialOrd for BoundLftPair {
    fn partial_cmp(&self, other: &BoundLftPair) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BoundLftPair {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.long_lft.cmp(&other.long_lft) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.outlived_lft.cmp(&other.outlived_lft),
        }
    }
}

fn get_declared_lifetimes(generics: &Generics<'_>) -> FxHashSet<Symbol> {
    generics
        .params
        .iter()
        .filter_map(|gp| {
            if let ParamName::Plain(ident) = gp.name {
                Some(ident.name)
            } else {
                None
            }
        })
        .collect()
}

fn get_declared_bounds(generics: &Generics<'_>) -> BTreeSet<BoundLftPair> {
    let mut declared_bounds = BTreeSet::new();
    for where_predicate in generics.predicates {
        match where_predicate {
            WherePredicate::RegionPredicate(region_predicate) => {
                let long_lft_sym = region_predicate.lifetime.ident.name;
                for generic_bound in region_predicate.bounds {
                    if let GenericBound::Outlives(outlived_lft) = *generic_bound {
                        declared_bounds.insert(BoundLftPair::new(long_lft_sym, outlived_lft.ident.name));
                    }
                }
            },
            WherePredicate::BoundPredicate(_) | WherePredicate::EqPredicate(_) => {},
        }
    }
    declared_bounds
}

struct ImpliedBoundsLinter {
    declared_lifetimes: FxHashSet<Symbol>,
    generics_span: Span,                     // for span_lint reporting
    declared_bounds: BTreeSet<BoundLftPair>, // BTreeSet for consistent reporting order
    implied_bounds: BTreeSet<BoundLftPair>,  // BTreeSet for consistent reporting order
}

impl ImpliedBoundsLinter {
    fn new(declared_lifetimes: FxHashSet<Symbol>, generics: &Generics<'_>) -> Self {
        ImpliedBoundsLinter {
            declared_lifetimes,
            declared_bounds: get_declared_bounds(generics),
            generics_span: generics.span,
            implied_bounds: BTreeSet::new(),
        }
    }

    fn declared_lifetime_sym(&self, region: Region<'_>) -> Option<Symbol> {
        let lft_sym_opt = region.get_name();
        if let Some(lft_sym) = lft_sym_opt
            && self.declared_lifetimes.contains(&lft_sym)
        {
            lft_sym_opt
        } else {
            None
        }
    }

    fn collect_implied_lifetimes_bounds(&mut self, ty: Ty<'_>) {
        self.collect_nested_ref_bounds(ty, None);
    }

    #[allow(rustc::usage_of_ty_tykind)]
    fn collect_nested_ref_bounds(&mut self, outliving_ty: Ty<'_>, outlived_lft_sym_opt: Option<Symbol>) {
        let mut outliving_tys = vec![outliving_ty];
        while let Some(ty) = outliving_tys.pop() {
            match *ty.kind() {
                TyKind::Ref(reference_region, referred_to_ty, _mutability) => {
                    if let Some(declared_lft_sym) = self.declared_lifetime_sym(reference_region) {
                        if let Some(outlived_lft_sym) = outlived_lft_sym_opt {
                            self.add_implied_bound(declared_lft_sym, outlived_lft_sym);
                        }
                        // ref_lft_sym should be outlived by referred_to_ty
                        self.collect_nested_ref_bounds(referred_to_ty, Some(declared_lft_sym));
                    } else {
                        outliving_tys.push(referred_to_ty);
                    }
                },
                TyKind::Tuple(tuple_part_tys) => {
                    for tuple_part_ty in tuple_part_tys {
                        outliving_tys.push(tuple_part_ty);
                    }
                },
                TyKind::Array(element_ty, _length) => {
                    outliving_tys.push(element_ty);
                },
                TyKind::Slice(element_ty) => {
                    outliving_tys.push(element_ty);
                },
                TyKind::Alias(AliasKind::Projection, alias_ty) => {
                    // For issue 10051: the for clause in: impl ... for ... {}
                    for alias_generic_arg in alias_ty.args {
                        if let Some(alias_ty) = alias_generic_arg.as_type() {
                            outliving_tys.push(alias_ty);
                        };
                    }
                },
                TyKind::Adt(_adt_def, generic_args) => {
                    // struct/union/enum
                    if let Some(outlived_lft_sym) = outlived_lft_sym_opt {
                        for generic_arg in generic_args {
                            if let Some(arg_region) = generic_arg.as_region()
                                && let Some(arg_lft_sym) = self.declared_lifetime_sym(arg_region)
                            {
                                self.add_implied_bound(arg_lft_sym, outlived_lft_sym);
                            }
                        }
                    }
                },
                _ => {},
            }
        }
    }

    fn add_implied_bound(&mut self, long_lft_sym: Symbol, outlived_lft_sym: Symbol) {
        if long_lft_sym != outlived_lft_sym {
            // only unequal symbols form a lifetime bound
            self.implied_bounds
                .insert(BoundLftPair::new(long_lft_sym, outlived_lft_sym));
        }
    }

    fn report_lints(self, cx: &LateContext<'_>) {
        for implied_bound in self.implied_bounds.difference(&self.declared_bounds) {
            span_lint(
                cx,
                IMPLICIT_LIFETIMES_BOUND_NESTED_REF,
                self.generics_span,
                &format!(
                    "missing lifetime bound declaration: {}",
                    implied_bound.as_bound_declaration()
                ),
            );
        }

        for declared_bound in self.declared_bounds.intersection(&self.implied_bounds) {
            span_lint(
                cx,
                EXPLICIT_LIFETIMES_BOUND_NESTED_REF,
                self.generics_span,
                &format!(
                    "declared lifetime bound is implied: {}",
                    declared_bound.as_bound_declaration()
                ),
            );
        }
    }
}

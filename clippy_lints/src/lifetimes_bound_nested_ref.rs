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
/// The lint here suggests to add such lifetime bounds in the hope that
/// the unsoundness is avoided.
///
/// There is also a reverse lint that suggest to remove lifetime bounds
/// that are implied by nested references. This reverse lint is intended to be used only
/// when the compiler has been fixed to handle these lifetime bounds correctly.
///
/// The lints here are in the nursery category.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use clippy_utils::diagnostics::{span_lint, span_lint_and_help, span_lint_and_sugg};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    Body, FnDecl, GenericArg as HirGenericArg, GenericBound, Generics, Item, ItemKind, ParamName, WherePredicate,
};
use rustc_hir_analysis::hir_ty_to_ty;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::ty_kind::TyKind;
use rustc_middle::ty::{BoundRegionKind, BoundVariableKind, ExistentialPredicate, GenericArg, List, Region, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol};

extern crate rustc_type_ir;
use rustc_type_ir::AliasKind;

extern crate rustc_hash;
use rustc_hash::FxHashMap;

declare_clippy_lint! {
    /// ### What it does
    /// For function arguments and return values and for implementation blocks
    /// this checks for nested references with generic lifetimes
    /// that imply a lifetimes bound because the inner reference must
    /// outlive the outer reference.
    /// This suggests to declare such implicit lifetime bounds.
    /// Adding such a bound helps to avoid unsound code because this addition
    /// can lead to a compiler error in related source code, as observed in rustc 1.76.0.
    ///
    /// ### Why is this bad?
    /// The unsoundness is described here:
    /// <https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md>.
    ///
    /// ### Known problems
    /// It is not known whether this covers all cases that might lead to unsoundness.
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
    #[clippy::version = "1.79.0"]
    pub EXPLICIT_LIFETIMES_BOUND,
    nursery,
    "declare generic lifetime bounds implied by nested references"
}

declare_clippy_lint! {
    /// ### What it does
    /// For function arguments and return values and implementation blocks
    /// this checks for nested references with generic lifetimes
    /// that imply a lifetimes bound because the inner reference must
    /// outlive the outer reference.
    /// This suggests to remove such implicit lifetime bounds when
    /// they are declared.
    ///
    /// ### Why is this bad?
    /// The declared lifetime bounds are superfluous.
    ///
    /// ### Known problems
    /// Removing such explicitly declared lifetime bounds may lead to the unsoundness described here:
    /// <https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md>.
    /// Removing these redundant lifetime bounds should only be done after the compiler
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

    #[clippy::version = "1.79.0"]
    pub IMPLICIT_LIFETIMES_BOUND,
    nursery,
    "remove declared generic lifetime bounds implied by nested references"
}

pub struct LifetimesBoundNestedRef;

impl_lint_pass!(LifetimesBoundNestedRef => [
    EXPLICIT_LIFETIMES_BOUND,
    IMPLICIT_LIFETIMES_BOUND,
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
        let declared_lifetimes = get_declared_lifetimes_spans(generics);
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
        let declared_lifetimes = get_declared_lifetimes_spans(impl_item.generics);
        if declared_lifetimes.len() <= 1 {
            return;
        }
        let mut linter = ImpliedBoundsLinter::new(declared_lifetimes, impl_item.generics);
        for path_segment in of_trait_ref.path.segments {
            if let Some(generic_args) = path_segment.args {
                for generic_arg in generic_args.args {
                    if let HirGenericArg::Type(hir_arg_ty) = generic_arg {
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
    long_lft_sym: Symbol,
    outlived_lft_sym: Symbol,
}

impl BoundLftPair {
    fn new(long_lft_sym: Symbol, outlived_lft_sym: Symbol) -> Self {
        BoundLftPair {
            long_lft_sym,
            outlived_lft_sym,
        }
    }

    fn as_bound_declaration(&self) -> String {
        format!("{}: {}", self.long_lft_sym, self.outlived_lft_sym)
    }
}

impl PartialEq for BoundLftPair {
    fn eq(&self, other: &Self) -> bool {
        self.long_lft_sym.eq(&other.long_lft_sym) && self.outlived_lft_sym.eq(&other.outlived_lft_sym)
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
        match self.long_lft_sym.cmp(&other.long_lft_sym) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.outlived_lft_sym.cmp(&other.outlived_lft_sym),
        }
    }
}

fn get_declared_lifetimes_spans(generics: &Generics<'_>) -> FxHashMap<Symbol, Span> {
    generics
        .params
        .iter()
        .filter_map(|gp| {
            if let ParamName::Plain(ident) = gp.name {
                Some((ident.name, gp.span))
            } else {
                None
            }
        })
        .collect()
}

fn get_declared_bounds_spans(generics: &Generics<'_>) -> BTreeMap<BoundLftPair, Span> {
    let mut declared_bounds = BTreeMap::new();
    for where_predicate in generics.predicates {
        match where_predicate {
            WherePredicate::RegionPredicate(region_predicate) => {
                let long_lft_sym = region_predicate.lifetime.ident.name;
                let long_lft_span = region_predicate.span;
                for generic_bound in region_predicate.bounds {
                    if let GenericBound::Outlives(outlived_lft) = *generic_bound {
                        declared_bounds.insert(BoundLftPair::new(long_lft_sym, outlived_lft.ident.name), long_lft_span);
                    }
                }
            },
            WherePredicate::BoundPredicate(_) | WherePredicate::EqPredicate(_) => {},
        }
    }
    declared_bounds
}

struct ImpliedBoundsLinter {
    declared_lifetimes_spans: FxHashMap<Symbol, Span>,
    generics_span: Span,
    declared_bounds_spans: BTreeMap<BoundLftPair, Span>, // BTree for consistent reporting order
    implied_bounds: BTreeSet<BoundLftPair>,              // BTree for consistent reporting order
}

impl ImpliedBoundsLinter {
    fn new(declared_lifetimes_spans: FxHashMap<Symbol, Span>, generics: &Generics<'_>) -> Self {
        ImpliedBoundsLinter {
            declared_lifetimes_spans,
            declared_bounds_spans: get_declared_bounds_spans(generics),
            generics_span: generics.span,
            implied_bounds: BTreeSet::new(),
        }
    }

    fn declared_lifetime_sym(&self, lft_sym_opt: Option<Symbol>) -> Option<Symbol> {
        lft_sym_opt.filter(|lft_sym| self.declared_lifetimes_spans.contains_key(lft_sym))
    }

    fn declared_lifetime_sym_region(&self, region: Region<'_>) -> Option<Symbol> {
        self.declared_lifetime_sym(region.get_name())
    }

    fn declared_lifetime_sym_bound_region(&self, bound_region: &BoundRegionKind) -> Option<Symbol> {
        self.declared_lifetime_sym(bound_region.get_name())
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
                    if let Some(region_sym) = self.declared_lifetime_sym_region(reference_region) {
                        if let Some(outlived_lft_sym) = outlived_lft_sym_opt {
                            self.add_implied_bound(region_sym, outlived_lft_sym);
                        }
                        self.collect_nested_ref_bounds(referred_to_ty, Some(region_sym));
                    } else {
                        outliving_tys.push(referred_to_ty);
                    }
                },
                TyKind::Tuple(tuple_part_tys) => {
                    // 20240328: not needed to detect reported issues
                    for tuple_part_ty in tuple_part_tys {
                        outliving_tys.push(tuple_part_ty);
                    }
                },
                TyKind::Array(element_ty, _length) => {
                    // 20240328: not needed to detect reported issues
                    outliving_tys.push(element_ty);
                },
                TyKind::Slice(element_ty) => {
                    // 20240328: not needed to detect reported issues
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
                    // struct/union/enum, 20240328: not needed to detect reported issues
                    if let Some(outlived_lft_sym) = outlived_lft_sym_opt {
                        self.collect_bounds_generic_args(generic_args, outlived_lft_sym);
                    }
                },
                TyKind::Dynamic(existential_predicates, dyn_region, _dyn_kind) => {
                    // dyn, 20240328: not needed to detect reported issues
                    if let Some(outlived_lft_sym) = outlived_lft_sym_opt {
                        for bound_existential_pred in existential_predicates {
                            match bound_existential_pred.skip_binder() {
                                ExistentialPredicate::Projection(exist_projection) => {
                                    self.collect_bounds_generic_args(exist_projection.args, outlived_lft_sym);
                                },
                                ExistentialPredicate::Trait(..) | ExistentialPredicate::AutoTrait(..) => {},
                            }
                            for bound_var_kind in bound_existential_pred.bound_vars() {
                                match bound_var_kind {
                                    BoundVariableKind::Region(bound_region_kind) => {
                                        if let Some(declared_lft_sym) =
                                            self.declared_lifetime_sym_bound_region(&bound_region_kind)
                                        {
                                            self.add_implied_bound(declared_lft_sym, outlived_lft_sym);
                                        }
                                    },
                                    BoundVariableKind::Ty(..) | BoundVariableKind::Const => {},
                                }
                            }
                        }
                        if let Some(declared_lft_sym) = self.declared_lifetime_sym_region(dyn_region) {
                            self.add_implied_bound(declared_lft_sym, outlived_lft_sym);
                        }
                    }
                },
                _ => {},
            }
        }
    }

    fn collect_bounds_generic_args(&mut self, generic_args: &List<GenericArg<'_>>, outlived_lft_sym: Symbol) {
        for generic_arg in generic_args {
            if let Some(region) = generic_arg.as_region()
                && let Some(declared_lft_sym) = self.declared_lifetime_sym_region(region)
            {
                self.add_implied_bound(declared_lft_sym, outlived_lft_sym);
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

    fn get_declared_lifetime_span(&self, lft_sym: Symbol) -> Option<Span> {
        self.declared_lifetimes_spans.get(&lft_sym).copied()
    }

    fn report_lints(self, cx: &LateContext<'_>) {
        for implied_bound in &self.implied_bounds {
            if !self.declared_bounds_spans.contains_key(implied_bound) {
                let declaration = implied_bound.as_bound_declaration();
                let msg = &format!("missing lifetimes bound declaration: {declaration}");
                if let Some(long_lft_decl_span) = self.get_declared_lifetime_span(implied_bound.long_lft_sym) {
                    span_lint_and_sugg(
                        cx,
                        EXPLICIT_LIFETIMES_BOUND,
                        long_lft_decl_span,
                        msg,
                        "try",
                        declaration,
                        Applicability::MachineApplicable,
                    );
                } else {
                    span_lint(cx, EXPLICIT_LIFETIMES_BOUND, self.generics_span, msg);
                }
            }
        }

        for (declared_bound, span) in self.declared_bounds_spans {
            if self.implied_bounds.contains(&declared_bound) {
                let help_span = None; // the span of the nested ref would be better
                span_lint_and_help(
                    cx,
                    IMPLICIT_LIFETIMES_BOUND,
                    span,
                    &format!(
                        "declared lifetimes bound is implied: {}",
                        declared_bound.as_bound_declaration(),
                    ),
                    help_span,
                    "consider removing this lifetimes bound",
                );
            }
        }
    }
}

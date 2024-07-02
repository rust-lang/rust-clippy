/// Lints to help dealing with unsoundness due to a compiler bug described here:
/// <https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md>.
///
/// For the following three cases the current compiler (1.76.0) gives a later error message when
/// declaring a generic lifetime bound that is implied by a nested reference:
///
///     [Issue 25860](https://github.com/rust-lang/rust/issues/25860):
///     Implied bounds on nested references + variance = soundness hole
///     
///     [Issue 84591](https://github.com/rust-lang/rust/issues/84591):
///     HRTB on subtrait unsoundly provides HTRB on supertrait with weaker implied bounds
///     
///     [Issue 100051](https://github.com/rust-lang/rust/issues/100051):
///     Implied bounds from projections in impl header can be unsound
///     
/// The lint here suggests to declare such lifetime bounds in the hope that
/// the unsoundness is avoided.
///
/// There is also a reverse lint that suggest to remove lifetime bounds
/// that are implied by nested references. This reverse lint is intended to be used only
/// when the compiler has been fixed to handle these lifetime bounds correctly.
use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use clippy_utils::diagnostics::{span_lint, span_lint_and_note, span_lint_and_then};
use rustc_ast::visit::FnKind;
use rustc_ast::{
    AngleBracketedArg, FnRetTy, GenericArg, GenericArgs, GenericBound, GenericParamKind, Generics, Item, ItemKind,
    NodeId, Path, Ty, TyKind, WherePredicate,
};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::Ident;
use rustc_span::{Span, Symbol};

extern crate rustc_hash;
use rustc_hash::FxHashMap;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for nested references with declared generic lifetimes
    /// in function arguments and return values and in implementation blocks.
    /// Such a nested reference implies a lifetimes bound because the inner reference must
    /// outlive the outer reference.
    ///
    /// This lint suggests to declare such implicit lifetime bounds in case they are not declared.
    /// Adding such a lifetimes bound helps to avoid unsound code because this addition
    /// can lead to a compiler error in related source code, as observed in rustc 1.76.0.
    ///
    /// The unusual way to use this lint is:
    /// 1) Set the lint to warn by this clippy command line argument:
    ///    ```--warn clippy::explicit-lifetimes-bound```
    ///    Without clippy errors, stop here.
    /// 2) Add the implied lifetime bound manually, or do this automatically with these command line arguments:
    ///    ```--fix --warn clippy::explicit-lifetimes-bound```
    ///    The code now has a declared explicit lifetimes bound that corresponds to the implied bound.
    /// 3) Run the compiler on the code with this declared lifetimes bound.
    ///    In case the compiler now produces a compiler error on related code,
    ///    the compiler should already have produced this error before declaring the implied bound.
    ///    Leave the added lifetimes bound in the code and fix the code producing the compiler error.
    ///
    /// See also the reverse lint clippy::implicit-lifetimes-bound.
    ///
    /// ### Why is this bad?
    /// The unsoundness is described
    /// [here](https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md).
    ///
    /// ### Known problems
    /// This lint tries to detect implied lifetime bounds for
    /// [issue 25860](https://github.com/rust-lang/rust/issues/25860),
    /// [issue 84591](https://github.com/rust-lang/rust/issues/84591), and
    /// [issue 100051](https://github.com/rust-lang/rust/issues/100051).
    /// It is not known whether this covers all cases that lead to unsoundness for implied lifetime bounds.
    ///
    /// The automatic fix is not extensively tested, so manually adding the implied lifetimes bound may be necessary.
    ///
    /// ### Example
    /// Here the type of the ```val_a``` argument contains ```&'a &'b``` which implies the lifetimes bound ```'b: 'a```:
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
    #[clippy::version = "1.81.0"]
    pub EXPLICIT_LIFETIMES_BOUND,
    nursery,
    "declare lifetime bounds implied by nested references"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for nested references with declared generic lifetimes
    /// in function arguments and return values and in implementation blocks.
    /// Such a nested reference implies a lifetimes bound because the inner reference must
    /// outlive the outer reference.
    ///
    /// This lint shows such implicit lifetime bounds in case they are declared.
    /// **WARNING:** Do not remove these lifetime bounds declararations, see "Known problems" below.
    ///
    /// See also the reverse lint clippy::explicit-lifetimes-bound.
    ///
    /// ### Why is this bad?
    /// The declared lifetime bounds are superfluous.
    ///
    /// ### Known problems
    /// This lint tries to detect implied lifetime bounds for
    /// [issue 25860](https://github.com/rust-lang/rust/issues/25860),
    /// [issue 84591](https://github.com/rust-lang/rust/issues/84591), and
    /// [issue 100051](https://github.com/rust-lang/rust/issues/100051).
    /// Removing the corresponding explicitly declared lifetime bounds may lead to the unsoundness described
    /// [here](https://github.com/rust-lang/rustc-dev-guide/blob/478a77a902f64e5128e7164e4e8a3980cfe4b133/src/traits/implied-bounds.md).
    ///
    /// Removing these redundant lifetime bounds should only be done after the compiler
    /// has been fixed to deal correctly with implied lifetime bounds.
    ///
    /// ### Example
    /// Here the type of the ```val_a``` argument contains ```&'a &'b``` which implies the lifetimes bound ```'b: 'a```:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b: 'a, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```
    /// Only after the compiler is fixed, use instead:
    /// ```no_run
    /// pub const fn lifetime_translator<'a, 'b, T>(val_a: &'a &'b (), val_b: &'b T) -> &'a T {
    ///     val_b
    /// }
    /// ```
    #[clippy::version = "1.79.0"]
    pub IMPLICIT_LIFETIMES_BOUND,
    nursery,
    "detect declared lifetime bounds implied by nested references"
}

pub struct LifetimesBoundNestedRef;

impl_lint_pass!(LifetimesBoundNestedRef => [
    EXPLICIT_LIFETIMES_BOUND,
    IMPLICIT_LIFETIMES_BOUND,
]);

impl EarlyLintPass for LifetimesBoundNestedRef {
    /// For issue 25860
    fn check_fn(&mut self, early_context: &EarlyContext<'_>, fn_kind: FnKind<'_>, _fn_span: Span, _node_id: NodeId) {
        let FnKind::Fn(_fn_ctxt, _ident, fn_sig, _visibility, generics, _opt_block) = fn_kind else {
            return;
        };
        let declared_lifetimes_spans = get_declared_lifetimes_spans(generics);
        if declared_lifetimes_spans.len() <= 1 {
            return;
        }
        let mut linter = ImpliedBoundsLinter::new(declared_lifetimes_spans, generics);
        for param in &fn_sig.decl.inputs {
            linter.collect_implied_lifetime_bounds(&param.ty);
        }
        if let FnRetTy::Ty(ret_ty) = &fn_sig.decl.output {
            linter.collect_implied_lifetime_bounds(ret_ty);
        }
        linter.report_lints(early_context);
    }

    /// For issues 84591 and 100051
    fn check_item_post(&mut self, early_context: &EarlyContext<'_>, item: &Item) {
        let ItemKind::Impl(box_impl) = &item.kind else {
            return;
        };
        let Some(of_trait_ref) = &box_impl.of_trait else {
            return;
        };
        let declared_lifetimes = get_declared_lifetimes_spans(&box_impl.generics);
        if declared_lifetimes.len() <= 1 {
            return;
        }
        let mut linter = ImpliedBoundsLinter::new(declared_lifetimes, &box_impl.generics);
        linter.collect_implied_lifetime_bounds_path(&of_trait_ref.path);
        // issue 10051 for clause: impl ... for for_clause_ty
        let for_clause_ty = &box_impl.self_ty;
        linter.collect_implied_lifetime_bounds(for_clause_ty);
        linter.report_lints(early_context);
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

fn get_declared_lifetimes_spans(generics: &Generics) -> FxHashMap<Symbol, Span> {
    generics
        .params
        .iter()
        .filter_map(|gp| {
            if let GenericParamKind::Lifetime = gp.kind {
                Some((gp.ident.name, gp.ident.span))
            } else {
                None
            }
        })
        .collect()
}

fn get_declared_bounds_spans(generics: &Generics) -> BTreeMap<BoundLftPair, Span> {
    let mut declared_bounds = BTreeMap::new();
    generics.params.iter().for_each(|gp| {
        let long_lft_sym = gp.ident.name;
        gp.bounds.iter().for_each(|bound| {
            if let GenericBound::Outlives(outlived_lft) = bound {
                let decl_span = if let Some(colon_span) = gp.colon_span {
                    spans_merge(colon_span, outlived_lft.ident.span)
                } else {
                    outlived_lft.ident.span
                };
                declared_bounds.insert(BoundLftPair::new(long_lft_sym, outlived_lft.ident.name), decl_span);
            }
        });
    });
    generics.where_clause.predicates.iter().for_each(|wp| {
        if let WherePredicate::RegionPredicate(wrp) = wp {
            let long_lft_sym = wrp.lifetime.ident.name;
            wrp.bounds.iter().for_each(|bound| {
                if let GenericBound::Outlives(outlived_lft) = bound {
                    // CHECKME: how to make a good span for the lifetimes bound declaration here?
                    declared_bounds.insert(BoundLftPair::new(long_lft_sym, outlived_lft.ident.name), wrp.span);
                }
            });
        }
    });
    declared_bounds
}

#[derive(Debug)]
struct ImpliedBoundsLinter {
    declared_lifetimes_spans: FxHashMap<Symbol, Span>,
    generics_span: Span,
    declared_bounds_spans: BTreeMap<BoundLftPair, Span>,
    implied_bounds_spans: BTreeMap<BoundLftPair, (Span, Span)>,
}

impl ImpliedBoundsLinter {
    fn new(declared_lifetimes_spans: FxHashMap<Symbol, Span>, generics: &Generics) -> Self {
        ImpliedBoundsLinter {
            declared_lifetimes_spans,
            declared_bounds_spans: get_declared_bounds_spans(generics),
            generics_span: generics.span,
            implied_bounds_spans: BTreeMap::new(),
        }
    }

    fn collect_implied_lifetime_bounds_path(&mut self, path: &Path) {
        self.collect_nested_ref_bounds_path(path, None);
    }

    fn collect_nested_ref_bounds_path(&mut self, path: &Path, opt_outlived_lft_ident: Option<&Ident>) {
        for path_segment in &path.segments {
            if let Some(generic_args) = &path_segment.args {
                if let GenericArgs::AngleBracketed(ab_args) = &**generic_args {
                    for ab_arg in &ab_args.args {
                        if let AngleBracketedArg::Arg(generic_arg) = ab_arg {
                            use GenericArg as GA;
                            match generic_arg {
                                GA::Lifetime(long_lft) => {
                                    if let Some(outlived_lft_ident) = opt_outlived_lft_ident
                                        && self.is_declared_lifetime_sym(long_lft.ident.name)
                                    {
                                        self.add_implied_bound_spans(&long_lft.ident, outlived_lft_ident);
                                    }
                                },
                                GA::Type(p_ty) => {
                                    self.collect_nested_ref_bounds(p_ty, opt_outlived_lft_ident);
                                },
                                GA::Const(_anon_const) => {},
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_implied_lifetime_bounds(&mut self, ty: &Ty) {
        self.collect_nested_ref_bounds(ty, None);
    }

    fn is_declared_lifetime_sym(&self, lft_sym: Symbol) -> bool {
        self.declared_lifetimes_spans.contains_key(&lft_sym)
    }

    fn collect_nested_ref_bounds(&mut self, outliving_ty: &Ty, opt_outlived_lft_ident: Option<&Ident>) {
        let mut outliving_tys = vec![outliving_ty]; // stack to avoid recursion
        while let Some(ty) = outliving_tys.pop() {
            use TyKind as TK;
            match &ty.kind {
                TK::Ref(opt_lifetime, referred_to_mut_ty) => {
                    // common to issues 25860, 84591 and 100051
                    let referred_to_ty = &referred_to_mut_ty.ty;
                    if let Some(lifetime) = opt_lifetime
                        && self.is_declared_lifetime_sym(lifetime.ident.name)
                    {
                        if let Some(outlived_lft_ident) = opt_outlived_lft_ident {
                            self.add_implied_bound_spans(&lifetime.ident, outlived_lft_ident);
                        }
                        // recursion for nested references outliving this lifetime
                        self.collect_nested_ref_bounds(referred_to_ty, Some(&lifetime.ident));
                    } else {
                        outliving_tys.push(referred_to_ty);
                    }
                },
                TK::Slice(element_ty) => {
                    // not needed to detect reported issues
                    outliving_tys.push(element_ty);
                },
                TK::Array(element_ty, _anon_const) => {
                    // not needed to detect reported issues
                    outliving_tys.push(element_ty);
                },
                TK::Tup(tuple_tys) => {
                    // not needed to detect reported issues
                    for tuple_ty in tuple_tys {
                        outliving_tys.push(tuple_ty);
                    }
                },
                TK::Path(opt_q_self, path) => {
                    if let Some(q_self) = opt_q_self {
                        // issue 100051
                        outliving_tys.push(&q_self.ty);
                    }
                    self.collect_nested_ref_bounds_path(path, opt_outlived_lft_ident);
                },
                TK::TraitObject(generic_bounds, _trait_object_syntax) => {
                    // dyn, not needed to detect reported issues
                    self.collect_nested_ref_bounds_gbs(generic_bounds, opt_outlived_lft_ident);
                },
                TK::ImplTrait(_node_id, generic_bounds, _opt_capturing_args_and_span) => {
                    // impl, not needed to detect reported issues
                    self.collect_nested_ref_bounds_gbs(generic_bounds, opt_outlived_lft_ident);
                },
                TK::AnonStruct(_node_id, _field_defs) | TK::AnonUnion(_node_id, _field_defs) => {
                    // CHECKME: can the field definition types of an anonymous struct/union have
                    // generic lifetimes?
                },
                TK::BareFn(_bare_fn_ty) => {
                    // CHECKME: can bare functions have generic lifetimes?
                },
                TK::CVarArgs
                | TK::Dummy
                | TK::Err(..)
                | TK::ImplicitSelf
                | TK::Infer
                | TK::MacCall(..)
                | TK::Never
                | TK::Pat(..)
                | TK::Paren(..)
                | TK::Ptr(..)
                | TK::Typeof(..) => {},
            }
        }
    }

    fn collect_nested_ref_bounds_gbs(
        &mut self,
        generic_bounds: &Vec<GenericBound>,
        opt_outlived_lft_ident: Option<&Ident>,
    ) {
        for gb in generic_bounds {
            use GenericBound as GB;
            match gb {
                GB::Trait(poly_trait_ref, _trait_bound_modifiers) => {
                    for bgp in &poly_trait_ref.bound_generic_params {
                        use GenericParamKind as GPK;
                        match &bgp.kind {
                            GPK::Lifetime => {
                                if let Some(outlived_lft_ident) = opt_outlived_lft_ident
                                    && self.is_declared_lifetime_sym(bgp.ident.name)
                                {
                                    self.add_implied_bound_spans(&bgp.ident, outlived_lft_ident);
                                }
                            },
                            GPK::Type { default: opt_p_ty } => {
                                if let Some(ty) = opt_p_ty {
                                    self.collect_nested_ref_bounds(ty, opt_outlived_lft_ident);
                                }
                            },
                            GPK::Const { ty, .. } => {
                                self.collect_nested_ref_bounds(ty, opt_outlived_lft_ident);
                            },
                        }
                    }
                },
                GB::Outlives(_lifetime) => {
                    // CHECKME: what is the meaning of GenericBound::Outlives ?
                },
            }
        }
    }

    fn add_implied_bound_spans(&mut self, long_lft_ident: &Ident, outlived_lft_ident: &Ident) {
        if long_lft_ident.name == outlived_lft_ident.name {
            // only unequal symbols form a lifetime bound
            return;
        }
        match self
            .implied_bounds_spans
            .entry(BoundLftPair::new(long_lft_ident.name, outlived_lft_ident.name))
        {
            Entry::Vacant(new_entry) => {
                // in nested references the outlived lifetime occurs first
                new_entry.insert((outlived_lft_ident.span, long_lft_ident.span));
            },
            Entry::Occupied(mut prev_entry) => {
                // keep the first occurrence of the nested reference,
                // the insertion order here depends on the recursion order.
                let prev_spans = prev_entry.get_mut();
                if (outlived_lft_ident.span < prev_spans.0)
                    || (outlived_lft_ident.span == prev_spans.0 && (long_lft_ident.span < prev_spans.1))
                {
                    *prev_spans = (outlived_lft_ident.span, long_lft_ident.span);
                }
            },
        }
    }

    fn report_lints(self, cx: &EarlyContext<'_>) {
        let bound_implied_here_note = "this lifetimes bound is implied here:";

        for (implied_bound, (outlived_lft_span, long_lft_span)) in &self.implied_bounds_spans {
            if !self.declared_bounds_spans.contains_key(implied_bound) {
                let declaration = implied_bound.as_bound_declaration();
                let msg_missing = format!("missing lifetimes bound declaration: {declaration}");
                if let Some(long_lft_decl_span) = self.declared_lifetimes_spans.get(&implied_bound.long_lft_sym) {
                    let nested_ref_span = spans_merge(*outlived_lft_span, *long_lft_span);
                    span_lint_and_fix_sugg_and_note_cause(
                        cx,
                        EXPLICIT_LIFETIMES_BOUND,
                        *long_lft_decl_span,
                        &msg_missing,
                        "try",
                        declaration,
                        nested_ref_span,
                        bound_implied_here_note,
                    );
                } else {
                    // unreachable!(); collected only bounds on declared lifetimes
                    span_lint(cx, EXPLICIT_LIFETIMES_BOUND, self.generics_span, msg_missing);
                }
            }
        }

        for (declared_bound, decl_span) in self.declared_bounds_spans {
            if let Some((outlived_lft_span, long_lft_span)) = self.implied_bounds_spans.get(&declared_bound) {
                let nested_ref_span = spans_merge(*outlived_lft_span, *long_lft_span);
                span_lint_and_note(
                    cx,
                    IMPLICIT_LIFETIMES_BOUND,
                    decl_span,
                    format!(
                        // only remove the these lifetime bounds after the compiler is fixed
                        "declared lifetimes bound: {} is redundant, but do not remove it",
                        declared_bound.as_bound_declaration(),
                    ),
                    Some(nested_ref_span),
                    bound_implied_here_note,
                );
            }
        }
    }
}

fn spans_merge(span1: Span, span2: Span) -> Span {
    Span::new(
        span1.lo().min(span2.lo()),
        span1.hi().max(span2.hi()),
        span1.ctxt(),
        span1.parent(),
    )
}

/// Combine `span_lint_and_sugg` and `span_lint_and_help`:
/// give a lint error, a suggestion to fix, and a note on the cause of the lint in the code.
#[allow(clippy::too_many_arguments)]
fn span_lint_and_fix_sugg_and_note_cause<T: LintContext>(
    cx: &T,
    lint: &'static Lint,
    sp: Span,
    msg: &str,
    fix_help: &str,
    sugg: String,
    cause_span: Span,
    cause_note: &'static str,
) {
    span_lint_and_then(cx, lint, sp, msg.to_owned(), |diag| {
        diag.span_suggestion(sp, fix_help.to_string(), sugg, Applicability::MachineApplicable);
        diag.span_note(cause_span, cause_note);
    });
}

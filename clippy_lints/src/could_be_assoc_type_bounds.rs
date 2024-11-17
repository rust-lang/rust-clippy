use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{SourceText, SpanRangeExt};
use clippy_utils::{SpanlessEq, WithSearchPat, is_from_proc_macro, over};
use itertools::Itertools;
use rustc_data_structures::fx::FxIndexMap;
use rustc_errors::{Applicability, MultiSpan};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::{self as hir, AssocItemConstraintKind, HirId, PathSegment, PredicateOrigin, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::symbol::Ident;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for trait bounds that constrain another trait bound's associated type,
    /// which could be expressed with [associated type bounds] directly.
    ///
    /// ### Why is this bad?
    /// Removing extra bounds and type parameters reduces noise and complexity
    /// and generally makes trait bounds easier to read.
    ///
    /// ### Example
    /// ```no_run
    /// fn example1<I>()
    /// where
    ///     I: Iterator,
    ///     <I as Iterator>::Item: Copy
    /// {}
    ///
    /// fn example2<I: Iterator<Item = T>, T: Copy>() {}
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn example1<I>()
    /// where
    ///     I: Iterator<Item: Copy>,
    /// {}
    ///
    /// fn example2<I: Iterator<Item: Copy>>() {}
    /// ```
    ///
    /// [associated type bounds]: https://blog.rust-lang.org/2024/06/13/Rust-1.79.0.html#bounds-in-associated-type-position
    #[clippy::version = "1.84.0"]
    pub COULD_BE_ASSOC_TYPE_BOUNDS,
    complexity,
    "trait bounds that could be expressed using associated type bounds"
}

#[derive(Debug)]
struct TyParamState {
    /// `DefId` of the type parameter
    def_id: LocalDefId,

    /// This count has two roles:
    /// - Any time this generic type parameter is referenced by a path within the item it is bound
    ///   at, this count is incremented.
    /// - When postprocessing an item's generics in `check_item_post`, anytime the generic parameter
    ///   is (1) the self type of a trait bound or (2) the associated type of a single other trait
    ///   bound, we decrement it.
    ///
    /// This gives us a cheap way to check if the type parameter is only used in ways that could be
    /// replaced with associated type bounds and not anywhere else without having to visit the
    /// item's HIR tree again in another visitor: if the count is zero after the decrementing
    /// phase (and `self.referenced_in_assoc_type` is true), then all uses of the generic parameter
    /// are valid uses and can be linted.
    /// Example:
    ///
    /// fn f<T: Iterator<Item = P>, P: Copy>() -> P {}
    ///                         ^   ^             ^
    ///
    /// After having visited everything in the item, the `use_count` of the generic parameter `P`
    /// is 3.
    /// We then check uses of `P` in the where clause and count 2 valid uses. `3 - 2 != 0` so at
    /// this point we see that we shouldn't lint.
    /// If the return type didn't mention `P` then the count would correctly be zero and we could
    /// suggest writing:
    ///
    /// fn f<T: Iterator<Item: Copy>>() {}
    use_count: u32,

    /// If this type parameter has been used to constrain an associated type in the where clause,
    /// then this stores the index into `trait_bounds` of that bound as well as the name of the
    /// associated type.
    constrained_assoc_type: Option<(usize, Ident)>,

    /// The index of this parameter into `hir::Generics::params`
    param_index: usize,
}

#[derive(Debug)]
struct ItemState {
    ty_params: Vec<TyParamState>,
    item_def_id: LocalDefId,
}

impl ItemState {
    fn new(item_def_id: LocalDefId, generics: &hir::Generics<'_>) -> Self {
        Self {
            item_def_id,
            ty_params: generics
                .params
                .iter()
                .enumerate()
                .filter_map(|(param_index, &hir::GenericParam { def_id, kind, .. })| {
                    if let hir::GenericParamKind::Type { synthetic: false, .. } = kind {
                        Some(TyParamState {
                            def_id,
                            param_index,
                            use_count: 0,
                            constrained_assoc_type: None,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    fn param_state(&mut self, def_id: LocalDefId) -> Option<&mut TyParamState> {
        self.ty_params.iter_mut().find(|t| t.def_id == def_id)
    }

    /// If the type is a type parameter bound by this item, return its state
    fn param_state_of_ty(&mut self, ty: &hir::Ty<'_>) -> Option<&mut TyParamState> {
        hir_ty_param(ty).and_then(|def_id| self.param_state(def_id))
    }

    /// Collects all `Type: Trait` bounds and finalizes the state of the type parameters
    /// (decrement the "use counts" of the type parameters along the way if in a valid position, see
    /// [`TyParamState::use_count`] for details).
    ///
    /// After this, all generic type parameters in `self.ty_params` that could be linted
    /// will have `use_count == 0 && constrained_assoc_type.is_some()`
    fn collect_trait_bounds_and_decrement_counts<'tcx>(
        &mut self,
        generics: &'tcx hir::Generics<'tcx>,
    ) -> Vec<TraitBound<'tcx>> {
        let mut trait_bounds = Vec::new();

        for (self_ty, generic_bounds, predicate_index) in type_trait_bounds(generics) {
            if let Some(ty_param) = self.param_state_of_ty(self_ty) {
                // (1) Used as the self type of a trait bound
                ty_param.use_count -= 1;
            }

            for generic_bound in generic_bounds {
                if let hir::GenericBound::Trait(poly_trait_ref) = generic_bound
                    && let [.., trait_path_segment] = poly_trait_ref.trait_ref.path.segments
                    && let Res::Def(DefKind::Trait, trait_def_id) = trait_path_segment.res
                    && trait_path_segment.args().parenthesized == hir::GenericArgsParentheses::No
                {
                    trait_bounds.push(TraitBound {
                        self_ty,
                        trait_def_id,
                        trait_path_segment,
                        emissions: Vec::new(),
                        span: generics.predicates[predicate_index].span(),
                    });

                    for constraint in trait_path_segment.args().constraints {
                        if let AssocItemConstraintKind::Equality { term } = constraint.kind
                            && let hir::Term::Ty(constrained_ty) = term
                            && let Some(ty_param) = self.param_state_of_ty(constrained_ty)
                            && ty_param.constrained_assoc_type.is_none()
                            // Make sure we don't lint something weird like `T: Trait<Assoc = T>` where the associated type is the same as the self type
                            && hir_ty_param(self_ty).is_none_or(|p| p != ty_param.def_id)
                        {
                            // (2) Used as the associated type of a trait bound **once**
                            ty_param.use_count -= 1;
                            ty_param.constrained_assoc_type = Some((trait_bounds.len() - 1, constraint.ident));
                        }
                    }
                }
            }
        }

        trait_bounds
    }

    /// Looks for trait bounds that constrain another trait bound's associated type once and add it
    /// to its `emissions` (delay a lint that is later emitted). Specifically, look for the
    /// following predicates:
    ///
    /// - `<T as Trait>::Assoc: Copy` if the bound `T: Trait` exists in `trait_bounds` and add
    ///   `Emission::ProjectionBound(<T as Trait>::Assoc: Copy)` to `T: Trait`s
    ///   [`TraitBound::emissions`]
    ///
    /// - `U: Trait` if exactly one `T: Trait<Assoc = U>` bound exists in `trait_bounds` (i.e.,
    ///   exactly one trait bound that has `U` as its associated type and is not used anywhere else,
    ///   ensured by `use_count` == 0). Add `Emission::TyParamBound(U: Trait)` to `T: Trait<Assoc =
    ///   U>`s [`TraitBound::emissions`].
    fn collect_emissions(
        &mut self,
        cx: &LateContext<'_>,
        trait_bounds: &mut [TraitBound<'_>],
        generics: &hir::Generics<'_>,
    ) {
        for (ty, trait_refs, predicate_index) in type_trait_bounds(generics) {
            let Some(bounds_span) = trait_refs
                .first()
                .zip(trait_refs.last())
                .map(|(first, last)| first.span().to(last.span()))
                .filter(|s| !s.from_expansion())
            else {
                continue;
            };

            if let hir::TyKind::Path(QPath::Resolved(Some(self_ty), path)) = ty.kind
                && let [.., trait_path, assoc_ty_path] = path.segments
                && let Res::Def(DefKind::Trait, projection_trait_def_id) = trait_path.res
                && let Res::Def(DefKind::AssocTy, _) = assoc_ty_path.res
                && let mut spanless_eq = SpanlessEq::new(cx).paths_by_resolution().inter_expr()
                && let Some(trait_bound) = trait_bounds.iter_mut().find(|t| {
                    t.trait_def_id == projection_trait_def_id
                        && spanless_eq.eq_ty(self_ty, t.self_ty)
                        // NB: intentionally don't check associated types
                        && over(
                            trait_path.args().args,
                            t.trait_path_segment.args().args,
                            |left, right| spanless_eq.eq_generic_arg(left, right)
                        )
                })
            {
                trait_bound.emissions.push(Emission::ProjectionBound {
                    predicate_index,
                    bounds_span,
                    assoc_type: assoc_ty_path.ident,
                });
            } else if let Some(ty_param_def_id) = hir_ty_param(ty)
                && let Some((ty_param_index, ty_param)) = self
                    .ty_params
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.def_id == ty_param_def_id)
                && ty_param.use_count == 0
                && let Some((trait_bound_index, _)) = ty_param.constrained_assoc_type
            {
                trait_bounds[trait_bound_index].emissions.push(Emission::TyParamBound {
                    predicate_index,
                    bounds_span,
                    ty_param_index,
                });
            }
        }
    }

    /// Emit lints for all previously collected [`TraitBound::emissions`].
    ///
    /// See the documentation for `TraitBound::emissions` for why we don't just immediately emit a
    /// warning in `ItemState::fill_emissions` and instead delay them for here.
    #[expect(clippy::too_many_lines)]
    fn lint_emissions(&mut self, cx: &LateContext<'_>, trait_bounds: &[TraitBound<'_>], generics: &hir::Generics<'_>) {
        for &TraitBound {
            self_ty: _,
            trait_def_id: _,
            trait_path_segment,
            span,
            ref emissions,
        } in trait_bounds.iter().filter(|b| !b.emissions.is_empty())
        {
            let (message, label) = if emissions.len() > 1 {
                (
                    "these trait bounds only exist to constrain another bound's associated type",
                    "merge them with this bound",
                )
            } else {
                (
                    "this trait bound only exists to constrain another bound's associated type",
                    "merge it with this bound",
                )
            };

            let mut spans = MultiSpan::from_spans(emissions.iter().map(|e| e.predicate(generics).span).collect());
            spans.push_span_label(span, label);

            span_lint_and_then(cx, COULD_BE_ASSOC_TYPE_BOUNDS, spans, message, |diag| {
                let mut suggestions = Vec::new();
                let mut removed_generic_param = false;

                // Group the emissions so we can merge all bounds from all predicates for
                // any single associated type at the same time
                let emissions = group_emissions_by_assoc_type(emissions, self);

                let exactly_one_where_bound = generics
                    .predicates
                    .iter()
                    .filter(|p| p.in_where_clause())
                    .exactly_one()
                    .is_ok();

                let exactly_one_generic_param = generics
                    .params
                    .iter()
                    .filter(|p| !p.is_elided_lifetime() && !p.is_impl_trait())
                    .exactly_one()
                    .is_ok();

                for emission in emissions.iter().flat_map(|(_, e)| e) {
                    // Only explicitly remove predicates that live in the where clause, or remove the whole where clause
                    // if this is the only one. Trait bounds in the generic parameter list don't
                    // need to be removed here as we remove the whole generic parameter including
                    // all bounds in one go further down.
                    if emission.predicate(generics).origin == PredicateOrigin::WhereClause {
                        if exactly_one_where_bound {
                            suggestions.push((generics.where_clause_span, String::new()));
                        } else {
                            suggestions.push((emission.predicate_span_including_comma(generics), String::new()));
                        }
                    }
                }

                if let Some(args) = trait_path_segment.args
                    && let Some(args_span) = args.span()
                {
                    let insert_span = args_span.shrink_to_hi();

                    // Generic arguments are present, insert `AssocTy: Bound1 + Bound2 + Bound3` for each associated
                    // type, or add them to the existing constraint
                    let mut new_bounds = String::new();

                    for (assoc, emissions) in emissions {
                        if let Some(constraint) = args.constraints.iter().find(|c| c.ident == assoc) {
                            // Associated type is already bound in the generics. Don't introduce
                            // a new associated type constraint and instead append bounds to the
                            // existing one.

                            match constraint.kind {
                                AssocItemConstraintKind::Equality { term } => {
                                    if let hir::Term::Ty(ty) = term
                                        && let Some(&mut TyParamState {
                                            param_index,
                                            constrained_assoc_type: Some(_),
                                            use_count: 0,
                                            ..
                                        }) = self.param_state_of_ty(ty)
                                    {
                                        // Remove the type parameter, including the following comma, or if this is the
                                        // only generic parameter, remove the whole generic argument list
                                        let removal_span = match generics.params.get(param_index..) {
                                            Some([own, next, ..])
                                                if !next.is_impl_trait() && !next.is_elided_lifetime() =>
                                            {
                                                own.span.until(next.span)
                                            },
                                            _ if exactly_one_generic_param => generics.span,
                                            _ => generics.params[param_index]
                                                .span
                                                .with_hi(generics.span.hi() - rustc_span::BytePos(1)),
                                        };
                                        suggestions.push((removal_span, String::new()));
                                        removed_generic_param = true;

                                        // Replace ` = P` of `Iterator<Item = P>` with `: Bounds`
                                        let mut sugg = String::from(": ");
                                        append_emission_bounds(&emissions, cx, &mut sugg, false);
                                        suggestions.push((constraint.span.with_lo(constraint.ident.span.hi()), sugg));
                                    }
                                },
                                AssocItemConstraintKind::Bound { bounds } => {
                                    let mut suggestion = String::new();
                                    append_emission_bounds(&emissions, cx, &mut suggestion, !bounds.is_empty());
                                    suggestions.push((constraint.span.shrink_to_hi(), suggestion));
                                },
                            }
                        } else {
                            new_bounds += ", ";
                            new_bounds += assoc.as_str();
                            new_bounds += ": ";

                            append_emission_bounds(&emissions, cx, &mut new_bounds, false);
                        }
                    }

                    if !new_bounds.is_empty() {
                        if let Some((_, sugg)) = suggestions.iter_mut().find(|(sp, _)| sp.hi() == insert_span.lo()) {
                            // rustfix considers replacements like `[(10:20, "foo"), (20:20, "bar")]` to have
                            // overlapping parts when they aren't overlapping. So work around it by
                            // extending the existing part in that case instead of pushing a new one.
                            *sugg += &new_bounds;
                        } else {
                            suggestions.push((insert_span, new_bounds));
                        }
                    }
                } else {
                    // Generic arguments not present
                    let mut new_bounds = String::from("<");

                    for (i, (assoc, emissions)) in emissions.iter().enumerate() {
                        if i > 0 {
                            new_bounds += ", ";
                        }

                        new_bounds += assoc.as_str();
                        new_bounds += ": ";

                        append_emission_bounds(emissions, cx, &mut new_bounds, false);
                    }

                    new_bounds += ">";
                    suggestions.push((trait_path_segment.ident.span.shrink_to_hi(), new_bounds));
                }

                diag.multipart_suggestion_verbose(
                    "remove any extra trait bounds add them directly to this trait bound using associated type bounds",
                    suggestions,
                    if removed_generic_param {
                        // Possibly requires changes at callsites
                        Applicability::Unspecified
                    } else {
                        Applicability::MachineApplicable
                    },
                );
            });
        }
    }

    fn postprocess_generics<'tcx>(
        mut self,
        cx: &LateContext<'tcx>,
        generics: &'tcx hir::Generics<'tcx>,
        item: &impl WithSearchPat<'tcx, Context = LateContext<'tcx>>,
    ) {
        // (Post)processing an item's generics is split into three parts (each one implemented as its own
        // method and documented in more detail):
        //
        // 1) Collect all trait bounds and decrement the use_counts of each type parameter
        // 2) Look for bounds that constrain another bound's associated type and add to its emissions if its
        //    use_count is 0
        // 3) Build suggestions and emit warnings for all of the collected `emissions` in step 2
        if generics.span.from_expansion() || is_from_proc_macro(cx, item) {
            return;
        }

        let mut trait_bounds = self.collect_trait_bounds_and_decrement_counts(generics);
        self.collect_emissions(cx, &mut trait_bounds, generics);
        self.lint_emissions(cx, &trait_bounds, generics);
    }
}

#[derive(Debug)]
enum Emission {
    ProjectionBound {
        predicate_index: usize,
        bounds_span: Span,
        assoc_type: Ident,
    },
    TyParamBound {
        predicate_index: usize,
        bounds_span: Span,
        /// Index into `ItemState::ty_params`
        ty_param_index: usize,
    },
}

impl Emission {
    fn predicate_index(&self) -> usize {
        match *self {
            Emission::ProjectionBound { predicate_index, .. } | Emission::TyParamBound { predicate_index, .. } => {
                predicate_index
            },
        }
    }

    fn predicate<'tcx>(&self, generics: &'tcx hir::Generics<'tcx>) -> &'tcx hir::WhereBoundPredicate<'tcx> {
        match &generics.predicates[self.predicate_index()] {
            hir::WherePredicate::BoundPredicate(bound) => bound,
            _ => unreachable!("this lint only looks for bound predicates"),
        }
    }

    fn predicate_span_including_comma(&self, generics: &hir::Generics<'_>) -> Span {
        let index = self.predicate_index();

        if let Some([own, next, ..]) = generics.predicates.get(index..)
            && next.in_where_clause()
        {
            own.span().until(next.span())
        } else {
            generics.predicates[index]
                .span()
                .until(generics.where_clause_span.shrink_to_hi())
        }
    }

    fn bounds_span(&self) -> Span {
        match *self {
            Emission::ProjectionBound { bounds_span, .. } | Emission::TyParamBound { bounds_span, .. } => bounds_span,
        }
    }

    fn assoc_ty(&self, item_state: &ItemState) -> Ident {
        match *self {
            Emission::ProjectionBound { assoc_type, .. } => assoc_type,
            Emission::TyParamBound { ty_param_index, .. } => {
                item_state.ty_params[ty_param_index]
                    .constrained_assoc_type
                    .expect("`Emission::TyParamBound` is only ever created for type parameters where `constrained_assoc_type.is_some()`")
                    .1
            },
        }
    }
}

fn group_emissions_by_assoc_type<'e>(
    emissions: &'e [Emission],
    item: &ItemState,
) -> FxIndexMap<Ident, Vec<&'e Emission>> {
    emissions.iter().fold(FxIndexMap::default(), |mut emissions, emission| {
        emissions.entry(emission.assoc_ty(item)).or_default().push(emission);
        emissions
    })
}

fn append_emission_bounds(
    emissions: &[&Emission],
    cx: &LateContext<'_>,
    out: &mut String,
    prepend_plus_at_start: bool,
) {
    for (i, emission) in emissions.iter().enumerate() {
        if i > 0 || prepend_plus_at_start {
            *out += " + ";
        }

        *out += emission
            .bounds_span()
            .get_source_text(cx)
            .as_ref()
            .map_or("..", SourceText::as_str);
    }
}

#[derive(Debug)]
struct TraitBound<'tcx> {
    self_ty: &'tcx hir::Ty<'tcx>,
    trait_def_id: DefId,
    trait_path_segment: &'tcx PathSegment<'tcx>,
    span: Span,
    /// We don't immediately emit lints when finding a predicate that can be merged with this one
    /// and instead delay it by pushing into this vec so we can build up a suggestion at the end
    /// once we know about all mergeable bounds, as otherwise the suggestion could end up with a
    /// broken suggestion when trying to insert `<Type: Trait>` twice. Example:
    ///
    ///     fn foo<T>()
    ///     where T: Iterator,
    ///         <T as Iterator>::Item: Copy + Sized,
    ///         <T as Iterator>::Item: Clone {}
    ///
    /// This should have one warning mentioning the last two bounds and suggest
    /// `T: Iterator<Item: Copy + Sized + Clone>`, instead of two warnings with
    /// both of them inserting `<Item: Copy + Sized>` and `<Item: Clone>`
    /// becoming `T: Iterator<Item: Copy + Sized><Item: Clone>`.
    ///
    /// It also overall simplifies the interaction between the two kinds of emissions in
    /// combination, e.g.
    ///
    ///     fn foo<T, U>()
    ///     where T: Iterator<Item = U>,
    ///         <T as Iterator>::Item: Copy + Sized,
    ///         U: Clone {}
    emissions: Vec<Emission>,
}

fn type_trait_bounds<'tcx>(
    generics: &'tcx hir::Generics<'tcx>,
) -> impl Iterator<Item = (&'tcx hir::Ty<'tcx>, &'tcx [hir::GenericBound<'tcx>], usize)> {
    generics
        .predicates
        .iter()
        .enumerate()
        .filter_map(|(predicate_index, predicate)| match predicate {
            hir::WherePredicate::BoundPredicate(predicate) => {
                Some((predicate.bounded_ty, predicate.bounds, predicate_index))
            },
            _ => None,
        })
}

fn hir_generics_of_item<'tcx>(item: &'tcx hir::Item<'tcx>) -> &'tcx hir::Generics<'tcx> {
    match item.kind {
        hir::ItemKind::Enum(_, generics)
        | hir::ItemKind::Fn(_, generics, _)
        | hir::ItemKind::Const(_, generics, _)
        | hir::ItemKind::Impl(&hir::Impl { generics, .. })
        | hir::ItemKind::Struct(_, generics)
        | hir::ItemKind::Trait(_, _, generics, ..)
        | hir::ItemKind::TraitAlias(generics, _)
        | hir::ItemKind::TyAlias(_, generics)
        | hir::ItemKind::Union(_, generics) => generics,
        _ => hir::Generics::empty(),
    }
}

fn hir_ty_param(ty: &hir::Ty<'_>) -> Option<LocalDefId> {
    if let hir::TyKind::Path(QPath::Resolved(None, path)) = ty.kind
        && let Res::Def(DefKind::TyParam, ty_param) = path.res
    {
        Some(ty_param.as_local().expect("type parameters are always crate local"))
    } else {
        None
    }
}

pub struct ManualAssocTypeBounds {
    msrv: Msrv,
    states: Vec<ItemState>,
}

impl ManualAssocTypeBounds {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv.clone(),
            states: Vec::new(),
        }
    }
}

impl_lint_pass!(ManualAssocTypeBounds => [COULD_BE_ASSOC_TYPE_BOUNDS]);

impl<'tcx> LateLintPass<'tcx> for ManualAssocTypeBounds {
    fn check_item(&mut self, _: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if self.msrv.meets(msrvs::ASSOCIATED_TYPE_BOUNDS) {
            self.states
                .push(ItemState::new(item.owner_id.def_id, hir_generics_of_item(item)));
        }
    }

    fn check_impl_item(&mut self, _: &LateContext<'tcx>, item: &'tcx hir::ImplItem<'tcx>) {
        if self.msrv.meets(msrvs::ASSOCIATED_TYPE_BOUNDS) {
            self.states.push(ItemState::new(item.owner_id.def_id, item.generics));
        }
    }

    fn check_impl_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::ImplItem<'tcx>) {
        if self.msrv.meets(msrvs::ASSOCIATED_TYPE_BOUNDS) {
            let state = self.states.pop().unwrap();
            debug_assert_eq!(state.item_def_id, item.owner_id.def_id);

            state.postprocess_generics(cx, item.generics, item);
        }
    }

    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if self.msrv.meets(msrvs::ASSOCIATED_TYPE_BOUNDS) {
            let state = self.states.pop().unwrap();
            debug_assert_eq!(state.item_def_id, item.owner_id.def_id);

            state.postprocess_generics(cx, hir_generics_of_item(item), item);
        }
    }

    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &hir::Path<'tcx>, _: HirId) {
        if let Res::Def(DefKind::TyParam, ty_param_def_id) = path.res
            && let ty_param_def_id = ty_param_def_id.expect_local()
            && let ty_param_bound_at = cx.tcx.parent(ty_param_def_id.to_def_id()).expect_local()
            && let Some(state) = self
                .states
                .iter_mut()
                .rev()
                .find(|s| s.item_def_id == ty_param_bound_at)
            && let Some(ty_param_count) = state.ty_params.iter_mut().find(|p| p.def_id == ty_param_def_id)
        {
            ty_param_count.use_count += 1;
        }
    }

    extract_msrv_attr!(LateContext);
}

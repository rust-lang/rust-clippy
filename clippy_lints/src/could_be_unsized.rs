use clippy_utils::diagnostics::span_lint;
use core::cell::Cell;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{GenericParam, Item, ItemKind, Node, TraitItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self, AliasTyKind, AssocContainer, AssocKind, Clause, ClauseKind, GenericArg, GenericArgKind, ParamTy, TermKind,
    Ty, TyCtxt, TypeFlags, TypeVisitableExt,
};
use rustc_session::impl_lint_pass;
use rustc_span::{DUMMY_SP, Span};

#[cfg(debug_assertions)]
use rustc_hir::def::DefKind;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for type parameters and associated types which could have a `?Sized` bound.
    ///
    /// ### Why is this bad?
    /// The lack or a `?Sized` bound unnecessarily restricts the types which can be used due to the
    /// implicit `Sized` bound.
    ///
    /// ### Example
    /// ```rust
    /// trait Foo<T> {
    ///     fn check(item: &T) -> bool;
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// trait Foo<T: ?Sized> {
    ///     fn check(item: &T) -> bool;
    /// }
    /// ```
    #[clippy::version = "1.60.0"]
    pub COULD_BE_UNSIZED,
    nursery,
    "item could have a `?Sized` bound"
}
impl_lint_pass!(CouldBeUnsized => [COULD_BE_UNSIZED]);

#[derive(Default)]
pub struct CouldBeUnsized {
    params: Params,
    dep_graph: DepGraph,
}

impl<'tcx> LateLintPass<'tcx> for CouldBeUnsized {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        match &item.kind {
            ItemKind::Struct(_, generics, ..) | ItemKind::Enum(_, generics, ..) | ItemKind::Union(_, generics, ..) => {
                self.eval_adt(cx.tcx, item.owner_id.to_def_id(), generics.params);
                let params_sizedness = self.params.param_cells(item.owner_id.to_def_id());
                report_params(cx, generics.params, params_sizedness);
            },
            ItemKind::Trait(_, _, _, _, generics, _, items) => {
                self.eval_trait(cx.tcx, item.owner_id.to_def_id(), generics.params);
                let params_sizedness = self.params.param_cells(item.owner_id.to_def_id());
                // The first sizedness result is for the `Self` type, not the first generic param.
                report_params(cx, generics.params, &params_sizedness[1..]);

                for assoc_item in items.iter().filter_map(|&i| {
                    let i = cx.tcx.hir_trait_item(i);
                    match i.kind {
                        TraitItemKind::Type(..) => Some(i),
                        _ => None,
                    }
                }) {
                    resolve_deps_for(
                        cx.tcx,
                        assoc_item.owner_id.to_def_id(),
                        &mut self.dep_graph,
                        &mut self.params,
                    );
                    let params_sizedness = self.params.param_cells(assoc_item.owner_id.to_def_id());
                    if let Some((item_sizedness, params_sizedness)) = params_sizedness.split_last() {
                        if item_sizedness.get().is_implicit_sized() {
                            span_lint(
                                cx,
                                COULD_BE_UNSIZED,
                                assoc_item.ident.span,
                                "associated type could have a `?Sized` bound",
                            );
                        }
                        report_params(cx, assoc_item.generics.params, params_sizedness);
                    }
                }
            },
            _ => (),
        }
    }
}

impl CouldBeUnsized {
    fn eval_trait(&mut self, tcx: TyCtxt<'_>, item: DefId, params: &[GenericParam<'_>]) {
        if !self.params.params.contains_key(&item) {
            load_trait(
                tcx,
                item,
                HirParams::from_trait_params(params),
                &mut self.dep_graph,
                &mut self.params,
            );
        }
        resolve_deps_for(tcx, item, &mut self.dep_graph, &mut self.params);
    }

    fn eval_adt(&mut self, tcx: TyCtxt<'_>, item: DefId, params: &[GenericParam<'_>]) {
        if !self.params.params.contains_key(&item) {
            load_adt_def(
                tcx,
                item,
                HirParams::from_params(params),
                &mut self.dep_graph,
                &mut self.params,
            );
        }
        resolve_deps_for(tcx, item, &mut self.dep_graph, &mut self.params);
    }
}

fn report_params(cx: &LateContext<'_>, params: &[GenericParam<'_>], params_sizedness: &[Cell<Sizedness>]) {
    for (param, _) in params
        .iter()
        .zip(params_sizedness.iter())
        .filter(|&(_, sizedness)| sizedness.get().is_implicit_sized())
    {
        span_lint(
            cx,
            COULD_BE_UNSIZED,
            param.name.ident().span,
            "generic param could have a `?Sized` bound",
        );
    }
}

#[derive(Default)]
struct Params {
    /// Stores the sizedness of each parameter for seen types, traits and trait associated types.
    params: DefIdMap<Box<[Cell<Sizedness>]>>,
}
impl Params {
    fn assoc_ty_param(&mut self, tcx: TyCtxt<'_>, item: DefId) -> (ParamId, &Cell<Sizedness>) {
        debug_assert!(tcx.def_kind(item) == DefKind::AssocTy);
        let params = self
            .params
            .entry(item)
            .or_insert_with(|| init_sizedness_for_assoc_ty(tcx, item));
        let index = params.len() - 1;
        #[allow(clippy::cast_possible_truncation)]
        (ParamId::new(item, index as u32), &params[index])
    }

    fn param(&self, item: ParamId) -> Sizedness {
        self.param_cell(item).get()
    }

    fn param_cell(&self, item: ParamId) -> &Cell<Sizedness> {
        &self.params[&item.did][item.index()]
    }

    fn param_cells(&self, item: DefId) -> &[Cell<Sizedness>] {
        &self.params[&item]
    }

    fn remove_or_init_assoc_ty(&mut self, tcx: TyCtxt<'_>, item: DefId) -> Box<[Cell<Sizedness>]> {
        self.params
            .remove(&item)
            .unwrap_or_else(|| init_sizedness_for_assoc_ty(tcx, item))
    }

    // fn remove_assoc_item(&mut self, item: DefId) -> Box<[Cell<Sizedness>]> {
    //     self.params.remove(&item).unwrap()
    // }

    fn insert(&mut self, item: DefId, sizedness: Box<[Cell<Sizedness>]>) {
        self.params.insert(item, sizedness);
    }
}

fn init_sizedness_for_assoc_ty(tcx: TyCtxt<'_>, id: DefId) -> Box<[Cell<Sizedness>]> {
    debug_assert!(tcx.def_kind(id) == DefKind::AssocTy);
    vec![Cell::new(Sizedness::Unsized); tcx.generics_of(id).own_params.len() + 1].into_boxed_slice()
}

fn init_sizedness_for_item(tcx: TyCtxt<'_>, id: DefId) -> Box<[Cell<Sizedness>]> {
    debug_assert!(tcx.def_kind(id) != DefKind::AssocTy);
    vec![Cell::new(Sizedness::Unsized); tcx.generics_of(id).own_params.len()].into_boxed_slice()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ParamId {
    did: DefId,
    index: u32,
}
impl ParamId {
    pub fn new(did: DefId, index: u32) -> Self {
        Self { did, index }
    }
    pub fn index(self) -> usize {
        self.index as usize
    }
}

#[derive(Clone, Copy)]
struct ItemRef<'a> {
    did: DefId,
    params_sizedness: &'a [Cell<Sizedness>],
}
impl<'a> ItemRef<'a> {
    fn new(did: DefId, params_sizedness: &'a [Cell<Sizedness>]) -> Self {
        Self { did, params_sizedness }
    }

    fn param_for_assoc_ty(self) -> (ParamId, &'a Cell<Sizedness>) {
        let index = self.params_sizedness.len() - 1;
        #[allow(clippy::cast_possible_truncation)]
        (ParamId::new(self.did, index as u32), &self.params_sizedness[index])
    }

    fn param(self, index: u32) -> (ParamId, &'a Cell<Sizedness>) {
        (ParamId::new(self.did, index), &self.params_sizedness[index as usize])
    }
}

#[derive(Clone, Copy)]
struct HirParams<'tcx> {
    offset: u8,
    params: &'tcx [GenericParam<'tcx>],
}
impl<'tcx> HirParams<'tcx> {
    fn param_span(&self, i: u32) -> Span {
        self.params
            .get((i as usize).wrapping_sub(self.offset.into()))
            .map_or(DUMMY_SP, |p| p.span)
    }

    fn from_params(params: &'tcx [GenericParam<'tcx>]) -> Self {
        Self { offset: 0, params }
    }

    fn from_trait_params(params: &'tcx [GenericParam<'tcx>]) -> Self {
        Self { offset: 1, params }
    }

    fn get_params(tcx: TyCtxt<'tcx>, id: DefId) -> &'tcx [GenericParam<'tcx>] {
        if let Some(Node::Item(item)) = tcx.hir_get_if_local(id)
            && let Some(generics) = item.kind.generics()
        {
            generics.params
        } else {
            [].as_slice()
        }
    }

    /// Gets the HIR params for the given item if it is a `LocalDefId`. Returns `DUMMY_PARAMS`
    /// otherwise. Note for traits use `from_trait_id` instead.
    fn from_def_id(tcx: TyCtxt<'tcx>, id: DefId) -> Self {
        debug_assert!(tcx.def_kind(id) != DefKind::Trait);
        Self {
            offset: 0,
            params: Self::get_params(tcx, id),
        }
    }

    /// Gets the HIR params for the given trait if it is a `LocalDefId`. Returns `DUMMY_PARAMS`
    /// otherwise.
    fn from_trait_id(tcx: TyCtxt<'tcx>, id: DefId) -> Self {
        debug_assert!(tcx.def_kind(id) == DefKind::Trait);
        Self {
            offset: 1,
            params: Self::get_params(tcx, id),
        }
    }
}

#[derive(Clone, Copy)]
struct AssocHirParams<'tcx> {
    name_span: Span,
    params: &'tcx [GenericParam<'tcx>],
}
impl<'tcx> AssocHirParams<'tcx> {
    // Used to avoid using Option<AssocHirParams>. Instead uses the bounds checking on the `params`
    // slice as it can't be removed by the compiler.
    const DUMMY_PARAMS: Self = Self {
        name_span: DUMMY_SP,
        params: [].as_slice(),
    };

    fn param_span(&self, i: u32) -> Span {
        self.params.get(i as usize).map_or(DUMMY_SP, |p| p.span)
    }

    /// Gets the HIR params for the given item if it is a `LocalDefId`. Returns `DUMMY_PARAMS`
    /// otherwise.
    fn from_def_id(tcx: TyCtxt<'tcx>, id: DefId) -> Self {
        debug_assert!(matches!(
            tcx.def_kind(id),
            DefKind::AssocConst | DefKind::AssocFn | DefKind::AssocTy
        ));

        if let Some(Node::TraitItem(item)) = tcx.hir_get_if_local(id) {
            Self {
                name_span: item.ident.span,
                params: item.generics.params,
            }
        } else {
            Self::DUMMY_PARAMS
        }
    }
}

/// The kind of item a dependency can be. This is a subset of `DefKind` used to avoid calls to
/// `TyCtxt::def_kind` and `DefIdTree::parent`.
#[derive(Clone, Copy)]
enum DependencyKind {
    Trait,
    Adt,
    AssocItem(DefId),
}

#[derive(Clone, Copy)]
struct Dependency {
    param: ParamId,
    kind: DependencyKind,
}
impl Dependency {
    fn new(param: ParamId, kind: DependencyKind) -> Self {
        Self { param, kind }
    }

    fn new_assoc_item(trait_id: DefId, param: ParamId) -> Self {
        Self {
            param,
            kind: DependencyKind::AssocItem(trait_id),
        }
    }
}

#[derive(Clone, Copy)]
struct ItemDependency {
    did: DefId,
    kind: DependencyKind,
}
impl ItemDependency {
    fn new(did: DefId, kind: DependencyKind) -> Self {
        Self { did, kind }
    }

    fn with_param(self, index: u32) -> Dependency {
        Dependency::new(ParamId::new(self.did, index), self.kind)
    }
}

#[derive(Clone, Copy)]
enum Sizedness {
    /// The parameter is not required to be sized.
    Unsized,
    /// The parameter is required to be sized by an implicit `Sized` bound only.
    ImplicitSized,
    /// The parameter is required to be sized by an explicit `Sized` bound, or by it's usage.
    Sized,
    /// The sizedness of the parameter depends on some other parameter. This will only exist during
    /// evaluation.
    HasDependency,
    /// The sizedness of the parameter depends on some other parameter, but it has an implicit
    /// `Sized` bound. This will only exist during evaluation.
    HasDependencyImplicitSized,
}
impl Sizedness {
    fn is_sized(self) -> bool {
        matches!(self, Self::Sized)
    }

    /// Is this sized only due to an implicit `Sized` bound.
    fn is_implicit_sized(self) -> bool {
        matches!(self, Self::ImplicitSized)
    }

    /// Add an implicit `Sized` bound.
    fn with_implicit_sized(self) -> Self {
        match self {
            Self::Unsized => Self::ImplicitSized,
            Self::HasDependency => Self::HasDependencyImplicitSized,
            _ => self,
        }
    }
}

/// Checks if the container of the associated type is the specified item, but only if the specified
/// item is a trait.
fn assoc_container_is_trait(tcx: TyCtxt<'_>, assoc_id: DefId, maybe_trait_id: DefId) -> bool {
    if matches!(tcx.associated_item(assoc_id).container, AssocContainer::Trait) {
        tcx.parent(assoc_id) == maybe_trait_id
    } else {
        false
    }
}

fn param_from_param_ty<'a>(ty: ParamTy, item: ItemRef<'a>, assoc_item: ItemRef<'a>) -> (ParamId, &'a Cell<Sizedness>) {
    if let Some(param_sizedness) = item.params_sizedness.get(ty.index as usize) {
        (ParamId::new(item.did, ty.index), param_sizedness)
    } else {
        debug_assert!(item.did != assoc_item.did);
        #[allow(clippy::cast_possible_truncation)]
        let param = ParamId::new(assoc_item.did, ty.index - item.params_sizedness.len() as u32);
        let param_sizedness = &assoc_item.params_sizedness[param.index()];
        (param, param_sizedness)
    }
}

type DepGraph = FxHashMap<ParamId, FxHashMap<ParamId, DependencyKind>>;

/// Adds a dependency to the given parameter, unless it is already `Sized`.
fn add_dep_to_param(param: ParamId, param_sizedness: &Cell<Sizedness>, dep: Dependency, dep_graph: &mut DepGraph) {
    match param_sizedness.get() {
        Sizedness::Sized => return,
        Sizedness::ImplicitSized => param_sizedness.set(Sizedness::HasDependencyImplicitSized),
        Sizedness::Unsized => param_sizedness.set(Sizedness::HasDependency),
        _ => (),
    }
    dep_graph.entry(param).or_default().insert(dep.param, dep.kind);
}

/// Adds a dependency to the given type if it is a generic parameter or an associated type of the
/// current trait. This will then continue to add any dependencies found within the type to the
/// dependency graph.
fn add_dep_to_ty<'tcx>(
    tcx: TyCtxt<'tcx>,
    dep: Dependency,
    ty: Ty<'tcx>,
    item: ItemRef<'_>,
    assoc_item: ItemRef<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    match *ty.kind() {
        ty::Param(ty) => {
            let (param, param_sizedness) = param_from_param_ty(ty, item, assoc_item);
            add_dep_to_param(param, param_sizedness, dep, dep_graph);
        },
        ty::Alias(AliasTyKind::Projection, ty) => {
            if let ty::Param(ParamTy { index: 0, .. }) = *ty.self_ty().kind() {
                if ty.def_id == assoc_item.did {
                    let (param, param_sizedness) = assoc_item.param_for_assoc_ty();
                    add_dep_to_param(param, param_sizedness, dep, dep_graph);
                } else if assoc_container_is_trait(tcx, ty.def_id, item.did) {
                    let (param, param_sizedness) = params.assoc_ty_param(tcx, ty.def_id);
                    add_dep_to_param(param, param_sizedness, dep, dep_graph);
                }
            }
        },
        _ => add_deps_from_ty(tcx, ty, item, assoc_item, dep_graph, params),
    }
}

/// Adds the sized constraint to a type if it is a generic parameter or an associated type of the
/// current trait. This will then continue to add any dependencies found within the type to the
/// dependency graph.
fn add_sized_constraint_to_ty<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    item: ItemRef<'_>,
    assoc_item: ItemRef<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    match *ty.kind() {
        ty::Param(ty) => {
            let (param, param_sizedness) = param_from_param_ty(ty, item, assoc_item);
            if !param_sizedness.get().is_sized() {
                param_sizedness.set(Sizedness::Sized);
                dep_graph.remove(&param);
            }
        },
        _ => add_deps_from_ty(tcx, ty, item, assoc_item, dep_graph, params),
    }
}

/// Adds any dependencies found in the type to the dependency graph.
fn add_deps_from_ty<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    item: ItemRef<'_>,
    assoc_item: ItemRef<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    if !ty.has_type_flags(TypeFlags::HAS_TY_PARAM | TypeFlags::HAS_TY_PROJECTION) {
        return;
    }

    match *ty.kind() {
        ty::Array(ty, _) | ty::Slice(ty) => add_sized_constraint_to_ty(tcx, ty, item, assoc_item, dep_graph, params),
        ty::Tuple(tys) => {
            for ty in tys {
                add_sized_constraint_to_ty(tcx, ty, item, assoc_item, dep_graph, params);
            }
        },
        ty::Adt(adt, args) => add_deps_from_args(
            tcx,
            ItemDependency::new(adt.did(), DependencyKind::Adt),
            args,
            item,
            assoc_item,
            dep_graph,
            params,
        ),
        ty::RawPtr(ty, _) | ty::Ref(_, ty, _) => {
            add_deps_from_ty(tcx, ty, item, assoc_item, dep_graph, params);
        },
        _ => (),
    }
}

/// Adds any dependencies found in the generic arguments to the dependency graph.
fn add_deps_from_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    dep: ItemDependency,
    args: &'tcx [GenericArg<'tcx>],
    item: ItemRef<'_>,
    assoc_item: ItemRef<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    for (i, ty) in args.iter().enumerate().filter_map(|(i, &arg)| {
        if let GenericArgKind::Type(ty) = arg.kind() {
            Some((i, ty))
        } else {
            None
        }
    }) {
        #[allow(clippy::cast_possible_truncation)]
        add_dep_to_ty(tcx, dep.with_param(i as u32), ty, item, assoc_item, dep_graph, params);
    }
}

/// Loads the predicates or a given item/assoc item into the dependency graph.
#[allow(clippy::too_many_arguments)]
fn load_predicates<'tcx>(
    tcx: TyCtxt<'tcx>,
    predicates: &'tcx [(Clause<'tcx>, Span)],
    item: ItemRef<'_>,
    hir_params: HirParams<'_>,
    assoc_item: ItemRef<'_>,
    assoc_hir_params: AssocHirParams<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    for (pred, pred_span) in predicates {
        match pred.kind().skip_binder() {
            ClauseKind::Trait(pred) => {
                let pred_trait_id = pred.def_id();
                if Some(pred_trait_id) == tcx.lang_items().sized_trait() {
                    // Check for an implicit `Sized` bound. This can only be done by checking if the `span` matches
                    // the span of the definition site.
                    let (param, param_sizedness, def_span) = match *pred.trait_ref.self_ty().kind() {
                        ty::Param(ty) => {
                            if let Some(param_sizedness) = item.params_sizedness.get(ty.index as usize) {
                                let param = ParamId::new(item.did, ty.index);
                                (param, param_sizedness, hir_params.param_span(ty.index))
                            } else {
                                #[allow(clippy::cast_possible_truncation)]
                                let (param, param_sizedness) =
                                    assoc_item.param(ty.index - item.params_sizedness.len() as u32);
                                (param, param_sizedness, assoc_hir_params.param_span(param.index))
                            }
                        },
                        ty::Alias(AliasTyKind::Projection, ty) => {
                            if let ty::Param(ParamTy { index: 0, .. }) = *ty.self_ty().kind() {
                                if ty.def_id == assoc_item.did {
                                    let (param, param_sizedness) = assoc_item.param_for_assoc_ty();
                                    (param, param_sizedness, assoc_hir_params.name_span)
                                } else if assoc_container_is_trait(tcx, ty.def_id, item.did) {
                                    let (param, param_sizedness) = params.assoc_ty_param(tcx, ty.def_id);
                                    (param, param_sizedness, DUMMY_SP)
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        },
                        _ => continue,
                    };
                    if *pred_span == def_span {
                        param_sizedness.set(param_sizedness.get().with_implicit_sized());
                    } else if !param_sizedness.get().is_sized() {
                        param_sizedness.set(Sizedness::Sized);
                        dep_graph.remove(&param);
                    }
                } else {
                    add_deps_from_args(
                        tcx,
                        ItemDependency::new(pred_trait_id, DependencyKind::Trait),
                        pred.trait_ref.args,
                        item,
                        assoc_item,
                        dep_graph,
                        params,
                    );
                }
            },
            ClauseKind::Projection(pred) => {
                if let TermKind::Ty(constraint_ty) = pred.term.kind() {
                    let (param, _) = params.assoc_ty_param(tcx, pred.projection_term.def_id);
                    let dep = Dependency::new_assoc_item(pred.projection_term.trait_def_id(tcx), param);
                    add_dep_to_ty(tcx, dep, constraint_ty, item, assoc_item, dep_graph, params);
                }
            },
            _ => (),
        }
    }
}

/// Loads the trait definition and all contained associated items into the parameter list and
/// dependency graph.
fn load_trait(
    tcx: TyCtxt<'_>,
    trait_id: DefId,
    hir_params: HirParams<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    debug_assert!(!params.params.contains_key(&trait_id));
    let params_sizedness = init_sizedness_for_item(tcx, trait_id);
    let item_ref = ItemRef::new(trait_id, &params_sizedness);
    load_predicates(
        tcx,
        tcx.explicit_predicates_of(trait_id).predicates,
        item_ref,
        hir_params,
        item_ref,
        AssocHirParams::DUMMY_PARAMS,
        dep_graph,
        params,
    );
    for item in tcx.associated_items(trait_id).in_definition_order() {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match item.kind {
            AssocKind::Fn { .. } => {
                debug_assert!(!params.params.contains_key(&item.def_id));
                let params_sizedness = init_sizedness_for_item(tcx, item.def_id);
                let assoc_item_ref = ItemRef::new(item.def_id, &params_sizedness);
                for ty in tcx.fn_sig(item.def_id).skip_binder().skip_binder().inputs_and_output {
                    add_sized_constraint_to_ty(tcx, ty, item_ref, assoc_item_ref, dep_graph, params);
                }
                params.insert(item.def_id, params_sizedness);
            },
            AssocKind::Type { .. } => {
                let params_sizedness = params.remove_or_init_assoc_ty(tcx, item.def_id);
                let assoc_item_ref = ItemRef::new(item.def_id, &params_sizedness);
                load_predicates(
                    tcx,
                    tcx.explicit_item_bounds(item.def_id).skip_binder(),
                    item_ref,
                    hir_params,
                    assoc_item_ref,
                    AssocHirParams::from_def_id(tcx, item.def_id),
                    dep_graph,
                    params,
                );
                params.insert(item.def_id, params_sizedness);
            },
            _ => (),
        }
    }
    params.insert(trait_id, params_sizedness);
}

/// Loads the adt definition into the parameter list and dependency graph.
fn load_adt_def(
    tcx: TyCtxt<'_>,
    adt_id: DefId,
    hir_params: HirParams<'_>,
    dep_graph: &mut DepGraph,
    params: &mut Params,
) {
    debug_assert!(!params.params.contains_key(&adt_id));
    let params_sizedness = init_sizedness_for_item(tcx, adt_id);
    let item_ref = ItemRef::new(adt_id, &params_sizedness);
    load_predicates(
        tcx,
        tcx.explicit_predicates_of(adt_id).predicates,
        item_ref,
        hir_params,
        item_ref,
        AssocHirParams::DUMMY_PARAMS,
        dep_graph,
        params,
    );
    for field in tcx.adt_def(adt_id).all_fields() {
        add_sized_constraint_to_ty(
            tcx,
            tcx.type_of(field.did).instantiate_identity(),
            item_ref,
            item_ref,
            dep_graph,
            params,
        );
    }
    params.insert(adt_id, params_sizedness);
}

/// Ensures the dependency has been loaded into the parameter list and dependency graph.
fn ensure_dep_is_loaded(tcx: TyCtxt<'_>, dep: Dependency, dep_graph: &mut DepGraph, params: &mut Params) {
    match dep.kind {
        DependencyKind::Adt if !params.params.contains_key(&dep.param.did) => {
            let hir_params = HirParams::from_def_id(tcx, dep.param.did);
            load_adt_def(tcx, dep.param.did, hir_params, dep_graph, params);
        },
        DependencyKind::Trait if !params.params.contains_key(&dep.param.did) => {
            let hir_params = HirParams::from_trait_id(tcx, dep.param.did);
            load_trait(tcx, dep.param.did, hir_params, dep_graph, params);
        },
        DependencyKind::AssocItem(trait_id) if !params.params.contains_key(&trait_id) => {
            let hir_params = HirParams::from_trait_id(tcx, trait_id);
            load_trait(tcx, trait_id, hir_params, dep_graph, params);
        },
        _ => (),
    }
}

// Resolves the sizedness of all parameters for a given item.
fn resolve_deps_for(tcx: TyCtxt<'_>, item: DefId, dep_graph: &mut DepGraph, params: &mut Params) {
    #[allow(clippy::cast_possible_truncation)]
    let param_count = params.param_cells(item).len() as u32;
    for param in (0..param_count).map(|i| ParamId::new(item, i)) {
        if matches!(
            params.param(param),
            Sizedness::HasDependency | Sizedness::HasDependencyImplicitSized
        ) {
            resolve_param(tcx, param, dep_graph, params);
        }
    }
}

/// Resolves the sizedness of a parameter from it's dependencies.
#[expect(rustc::potential_query_instability)]
fn resolve_param(tcx: TyCtxt<'_>, param: ParamId, dep_graph: &mut DepGraph, params: &mut Params) {
    // `deps` contains all unresolved dependencies encountered so far. This has the same order as
    // `param_stack`, so if `param_stack` contains `[(param1, 4), (param2, 4), (param3, 1)]` then `deps`
    // would contain `[(4 deps for param1), (4 deps for param2), (1 dep for param3)]`

    // `param_stack` contains the dependency chain from `param` all the way to the current parameter.
    // This is used to propagate `Sized` all the way from the current parameter up to `param`.

    // `visited_params` contains every parameter which has been checked. This is needed due to possible
    // cycles.

    let mut deps: Vec<_> = dep_graph
        .remove(&param)
        .unwrap()
        .into_iter()
        .map(|(param, kind)| Dependency::new(param, kind))
        .collect();
    let mut param_stack = vec![(param, deps.len())];
    let mut visited_params = FxHashSet::default();
    visited_params.insert(param);

    while let Some(dep) = deps.pop() {
        // No need to check for overflow here. The top item in `param_stack` will always have at least one
        // dependency. Either the removal code at the end of the loop removed everything from the top with
        // no dependencies left, or a new item was added to the top which has at least one dependency.
        let (_, count) = param_stack.last_mut().unwrap();
        *count -= 1;
        let mut count = *count;

        if visited_params.insert(dep.param) {
            ensure_dep_is_loaded(tcx, dep, dep_graph, params);
            match params.param(dep.param) {
                Sizedness::Sized | Sizedness::ImplicitSized | Sizedness::HasDependencyImplicitSized => {
                    // Propagate the sizedness all the way down the dependency chain.
                    for (param, _) in param_stack {
                        params.param_cell(param).set(Sizedness::Sized);
                        dep_graph.remove(&param);
                    }
                    return;
                },
                Sizedness::HasDependency => {
                    let param_deps = &dep_graph[&dep.param];
                    let dep_count = deps.len();
                    deps.extend(param_deps.iter().map(|(&param, &kind)| Dependency::new(param, kind)));
                    debug_assert!(dep_count < deps.len());
                    param_stack.push((dep.param, deps.len() - dep_count));

                    // Don't remove anything from `param_stack` even if the count is zero. The parameter is still part
                    // of the dependency chain.
                    continue;
                },
                Sizedness::Unsized => (),
            }
        }

        // Remove all parameters which no longer have dependencies to check.
        while count == 0 {
            param_stack.pop();
            let Some(&(_, next_count)) = param_stack.last() else {
                break;
            };
            count = next_count;
        }
    }

    // This can only happen in the case where all dependencies lead to cycles or unsized parameters.
    for param in visited_params {
        let param_sizedness = params.param_cell(param);
        match param_sizedness.get() {
            Sizedness::HasDependency => param_sizedness.set(Sizedness::Unsized),
            Sizedness::HasDependencyImplicitSized => param_sizedness.set(Sizedness::ImplicitSized),
            Sizedness::Unsized => (),
            Sizedness::Sized | Sizedness::ImplicitSized => debug_assert!(false),
        }
        dep_graph.remove(&param);
    }
}

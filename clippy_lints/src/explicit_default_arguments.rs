use std::iter;

use clippy_utils::MaybePath;
use clippy_utils::diagnostics::span_lint_hir;
use clippy_utils::source::snippet;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{
    self as hir, AssocItemConstraint, AssocItemConstraintKind, Closure, EnumDef, Expr, ExprKind, FnDecl, FnPtrTy,
    FnRetTy, FnSig, GenericArgs, GenericParam, GenericParamKind, Generics, Impl, ImplItemKind, Item, ItemKind, LetExpr,
    LetStmt, MutTy, OpaqueTy, OwnerId, PatKind, Path, PathSegment, QPath, StmtKind, Term, TraitItemKind, TyKind,
    Variant,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self, AliasTy, ExistentialPredicate, ExistentialProjection, ExistentialTraitRef, GenericArg, GenericParamDef,
    TraitPredicate, TraitRef, Ty, TyCtxt,
};

use rustc_session::declare_lint_pass;
use rustc_span::Ident;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of a generic argument when the type already defines a default.
    ///
    /// ### Why is this bad?
    /// It is redundant and adds visual clutter.
    ///
    /// ### Example
    /// ```no_run
    /// type Result<T = ()> = core::result::Result<T, MyError>;
    /// fn foo() -> Result<()> {
    ///     Ok(())
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// type Result<T = ()> = core::result::Result<T, MyError>;
    /// fn foo() -> Result {
    ///     Ok(())
    ///}
    /// ```
    #[clippy::version = "1.90.0"]
    pub EXPLICIT_DEFAULT_ARGUMENTS,
    style,
    "default lint description"
}

declare_lint_pass!(ExplicitDefaultArguments => [EXPLICIT_DEFAULT_ARGUMENTS]);

/// Map 1: Iterates through the aliased type's generic args and finds the defaults given by the type
/// alias definition. Returns a map of the index in the aliased ty's generics args to its found
/// default.
///
/// Map 2: Iterates through the aliased type's generics args and finds the index of it in the
/// actual alias definition. Returns a map of the index in the aliased type's generics args to the
/// corresponding index in the alias definition's generics params.
fn match_generics<'tcx>(
    cx: &LateContext<'tcx>,
    aliased_ty_args: &[GenericArg<'tcx>],
    alias_ty_params: &[GenericParamDef],
) -> (FxHashMap<usize, Ty<'tcx>>, FxHashMap<usize, u32>) {
    aliased_ty_args
        .iter()
        .enumerate()
        .filter_map(|(i, generic_arg)| {
            generic_arg.as_type().and_then(|ty| {
                if let ty::Param(param) = ty.kind() {
                    Some((i, param))
                } else {
                    None
                }
            })
        })
        .fold(
            (FxHashMap::default(), FxHashMap::default()),
            |(mut map1, mut map2), (i, param)| {
                if let Some(alias_ty_param) = alias_ty_params.iter().find(|param_def| param_def.name == param.name)
                    && let Some(default_value) = alias_ty_param
                        .default_value(cx.tcx)
                        .and_then(|default_value| default_value.skip_binder().as_type())
                {
                    map1.insert(i, default_value);
                    map2.insert(i, alias_ty_param.index);
                }
                (map1, map2)
            },
        )
}

fn check_alias_args<'tcx>(cx: &LateContext<'tcx>, resolved_ty: Ty<'tcx>, hir_ty: hir::Ty<'tcx>) {
    let (alias_ty_params, aliased_ty_args, hir_ty_args) = {
        let TyKind::Path(
            qpath @ QPath::Resolved(
                _,
                Path {
                    segments:
                        [
                            ..,
                            PathSegment {
                                args: Some(hir_ty_generics),
                                ..
                            },
                        ],
                    ..
                },
            ),
        ) = hir_ty.kind
        else {
            return;
        };
        let Res::Def(DefKind::TyAlias, alias_def_id) = cx.qpath_res(&qpath, hir_ty.hir_id()) else {
            // The ty doesn't refer to a type alias
            return;
        };
        let aliased_ty = cx.tcx.type_of(alias_def_id).skip_binder();
        // println!("aliased ty: {aliased_ty}");
        let ty::Adt(_, aliased_ty_args) = aliased_ty.kind() else {
            // The ty alias doesn't refer to an ADT
            return;
        };
        (
            &cx.tcx.generics_of(alias_def_id).own_params,
            aliased_ty_args,
            hir_ty_generics,
        )
    };
    let ty::Adt(_, resolved_ty_args) = resolved_ty.kind() else {
        return;
    };
    let (defaults, aliased_to_alias) = match_generics(cx, aliased_ty_args.as_slice(), alias_ty_params.as_slice());

    for (i, generic_arg) in resolved_ty_args.iter().enumerate() {
        // Was the default explicitly written, or was it there because just because it got resolved?
        // If something was specified and the resolved form of the type alias had the default,
        // then it is redundant
        if let Some(redundant_ty) = aliased_to_alias.get(&i).and_then(|i| hir_ty_args.args.get(*i as usize))
            && defaults.get(&i).copied() == generic_arg.as_type()
        {
            // TODO: show a hint
            span_lint_hir(
                &cx,
                EXPLICIT_DEFAULT_ARGUMENTS,
                redundant_ty.hir_id(),
                redundant_ty.span(),
                "redudant usage of default argument",
            );
        }
    }

    return;
}

type TyPair<'a> = (Ty<'a>, hir::Ty<'a>);

fn get_tys_fn_sig<'tcx>(
    tcx: TyCtxt<'tcx>,
    sig: FnSig<'tcx>,
    item_owner_id: OwnerId,
) -> impl IntoIterator<Item = TyPair<'tcx>> + 'tcx {
    // Assumes inputs are in the same order in `rustc_middle` and the hir.

    let poly_fn_sig = tcx.fn_sig(item_owner_id).skip_binder();

    let output_ty = poly_fn_sig.output().skip_binder();
    let output = if let FnRetTy::Return(output_hir_ty) = sig.decl.output {
        vec![(output_ty, *output_hir_ty)]
    } else {
        Vec::new()
    };
    let inputs_ty = poly_fn_sig.inputs().skip_binder();
    let inputs_hir_tys = sig.decl.inputs;
    inputs_ty
        .iter()
        .copied()
        .zip(inputs_hir_tys.iter().copied())
        .chain(output)
}
/// Get all types in the the generics.
/// Limitation: this does not look at generic predicates, such as the where clause, due to the added
/// complexity. This could change in the future.
fn get_tys_from_generics<'tcx>(
    tcx: TyCtxt<'tcx>,
    generics: &Generics<'tcx>,
    item_owner_id: OwnerId,
) -> impl IntoIterator<Item = TyPair<'tcx>> + 'tcx {
    // Assumes the generics are the same order in `rustc_middle` and the hir.
    let default_tys = tcx
        .generics_of(item_owner_id)
        .own_params
        .iter()
        .filter_map(move |param| {
            param
                .default_value(tcx)
                .and_then(|default_value| default_value.skip_binder().as_type())
        })
        .zip(
            generics
                .params
                .iter()
                .filter_map(|GenericParam { kind, .. }| match kind {
                    GenericParamKind::Type {
                        default: Some(default), ..
                    } => Some(**default),
                    GenericParamKind::Const { ty, .. } => Some(**ty),
                    _ => None,
                }),
        );
    // Might be a good idea to look at
    // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_hir_analysis/collect/predicates_of.rs.html
    // if checking predicates is ever implemented. The
    // [`ParamEnv`](https://rustc-dev-guide.rust-lang.org/typing_parameter_envs.html) is where the predicates would be
    // found in the `rustc_middle` level.

    default_tys
}

/// Creates a TypePair for every equality constraint by using the provided index. The index is
/// required because ordering in the HIR and rustc_middle do not match, so checking the assoc ty is
/// required.
fn get_constraint_types<'tcx, T: IntoIterator<Item = &'tcx hir::TraitRef<'tcx>>>(
    refs: T,
    ident_index: FxHashMap<Ident, (Ty<'tcx>, impl IntoIterator<Item = Ty<'tcx>> + Clone)>,
) -> Vec<TyPair<'tcx>> {
    // As usual, we are assuming that GATs are in the same order in the HIR and rustc_middle.
    refs.into_iter()
        .filter_map(|trait_ref| {
            if let hir::TraitRef {
                path:
                    Path {
                        segments:
                            [
                                ..,
                                PathSegment {
                                    args: Some(GenericArgs { constraints, .. }),
                                    ..
                                },
                            ],
                        ..
                    },
                ..
            } = trait_ref
            {
                Some(
                    constraints
                        .iter()
                        .filter_map(
                            |AssocItemConstraint {
                                 ident,
                                 kind,
                                 gen_args: hir_gen_args,
                                 ..
                             }| {
                                if let AssocItemConstraintKind::Equality { term: Term::Ty(hir_ty) } = kind {
                                    ident_index.get(ident).map(|(ty, gen_args)| {
                                        iter::once((*ty, **hir_ty)).chain(gen_args.clone().into_iter().zip(
                                            hir_gen_args.args.iter().filter_map(|arg| {
                                                if let hir::GenericArg::Type(ty) = arg {
                                                    Some(*ty.as_unambig_ty())
                                                } else {
                                                    None
                                                }
                                            }),
                                        ))
                                    })
                                } else {
                                    None
                                }
                            },
                        )
                        .flatten(),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

fn path_generic_args<'hir>(
    segments: impl IntoIterator<Item = &'hir PathSegment<'hir>>,
) -> impl IntoIterator<Item = hir::Ty<'hir>> {
    Box::new(
        segments
            .into_iter()
            .filter_map(|PathSegment { args, .. }| {
                args.map(|args| {
                    args.args.iter().filter_map(|arg| match arg {
                        hir::GenericArg::Type(ty) => Some(*ty.as_unambig_ty()),
                        _ => None,
                    })
                })
            })
            .flatten(),
    )
}

// TODO: move this to clippy_utils?
/// Walks `Ty` and `hir::Ty` in lockstep. Only use with type pairs that came from outside bodies,
/// e.g. function definitions.
fn walk_ty_recursive<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    hir_ty: hir::Ty<'tcx>,
) -> impl IntoIterator<Item = TyPair<'tcx>> {
    let trait_ref_args = |trait_ref: &hir::TraitRef<'tcx>| {
        if let hir::TraitRef {
            path:
                Path {
                    segments:
                        [
                            ..,
                            PathSegment {
                                args: Some(GenericArgs { args, .. }),
                                ..
                            },
                        ],
                    ..
                },
            ..
        } = trait_ref
        {
            Some(args.iter().filter_map(|arg| {
                if let hir::GenericArg::Type(ty) = arg {
                    Some(*ty.as_unambig_ty())
                } else {
                    None
                }
            }))
        } else {
            None
        }
    };
    let tys: Box<dyn Iterator<Item = TyPair<'tcx>>> = match (ty.kind(), hir_ty.kind) {
        (ty::Adt(_, generics), TyKind::Path(QPath::Resolved(None, path))) => Box::new(
            generics
                .iter()
                .filter_map(|arg| arg.as_type())
                .zip(path_generic_args(path.segments)),
        ),
        (ty::Array(ty, _), TyKind::Array(hir_ty, _))
        | (ty::Slice(ty), TyKind::Slice(hir_ty))
        | (ty::RawPtr(ty, _), TyKind::Ptr(MutTy { ty: hir_ty, .. }))
        | (ty::Ref(_, ty, _), TyKind::Ref(_, MutTy { ty: hir_ty, .. })) => Box::new(iter::once((*ty, *hir_ty))),
        (
            ty::FnPtr(tys, _),
            TyKind::FnPtr(FnPtrTy {
                decl:
                    FnDecl {
                        inputs: inputs_hir,
                        output: output_hir,
                        ..
                    },
                ..
            }),
        ) => {
            let tys = tys.skip_binder();
            let iter = tys.inputs().iter().copied().zip(inputs_hir.iter().copied());
            if let FnRetTy::Return(hir_ty) = output_hir {
                Box::new(iter.chain(iter::once((tys.output(), **hir_ty))))
            } else {
                Box::new(iter::empty())
            }
        },
        (ty::Dynamic(predicates, _, _), TyKind::TraitObject(hir_predicates, _)) => {
            // Assumes that generics in `rustc_middle` are in the same order as the hir.
            let trait_generics = predicates
                .iter()
                .filter_map(move |predicate| {
                    if let ExistentialPredicate::Trait(ExistentialTraitRef { args, .. }) = predicate.skip_binder() {
                        Some(args.iter().filter_map(|arg| arg.as_type()))
                    } else {
                        None
                    }
                })
                .flatten()
                .zip(
                    hir_predicates
                        .iter()
                        .map(|poly_trait_ref| &poly_trait_ref.trait_ref)
                        .filter_map(trait_ref_args)
                        .flatten(),
                );
            let ident_index = predicates
                .iter()
                .filter_map(move |predicate| {
                    if let ExistentialPredicate::Projection(ExistentialProjection { def_id, term, .. }) =
                        predicate.skip_binder()
                        && let Some(ty) = term.as_type()
                    {
                        // TODO: Check GATs if they ever become object safe (they aren't checked right now).
                        Some((tcx.item_ident(def_id), (ty, [].iter().copied())))
                    } else {
                        None
                    }
                })
                .collect();
            // The equality constraints will not have the same order, so this will match the identifiers of the
            // associated item.
            let trait_preds = get_constraint_types(
                hir_predicates.iter().map(|poly_trait_ref| &poly_trait_ref.trait_ref),
                ident_index,
            );

            Box::new(trait_generics.chain(trait_preds.into_iter()))
        },
        (ty::Alias(ty::Projection, AliasTy { args, .. }), TyKind::Path(QPath::TypeRelative(hir_ty, path_segment))) => {
            let hir_gat_args_iter = path_generic_args(iter::once(path_segment));
            Box::new(
                args.iter()
                    .flat_map(|arg| arg.as_type())
                    .zip(iter::once(*hir_ty).chain(hir_gat_args_iter)),
            )
        },
        // Same as the TypeRelative version for projections except that we have a trait. Something like `<T as
        // Trait>::Assoc::<..>`
        (
            ty::Alias(ty::Projection, AliasTy { args, .. }),
            TyKind::Path(QPath::Resolved(
                Some(hir_ty),
                Path {
                    segments:
                        [
                            // Since this is a projection, there really should be only 2 elements
                            hir_trait_path_segment,
                            ..,
                            hir_gat_path_segment,
                        ],
                    ..
                },
            )),
        ) => {
            // Assumes both will have the same order in `rustc_middle` and the hir
            let hir_trait_args_iter = path_generic_args(iter::once(hir_trait_path_segment));
            let hir_gat_args_iter = path_generic_args(iter::once(hir_gat_path_segment));
            Box::new(
                args.iter()
                    .flat_map(|arg| arg.as_type())
                    .zip(iter::once(*hir_ty).chain(hir_trait_args_iter).chain(hir_gat_args_iter)),
            )
        },
        (
            // `args` doesn't seem to have anything useful, not 100% sure.
            ty::Alias(ty::Opaque, AliasTy { args: _, def_id, .. }),
            TyKind::OpaqueDef(OpaqueTy {
                bounds: [hir_bounds @ ..],
                ..
            }),
        )
        | (ty::Alias(ty::Opaque, AliasTy { args: _, def_id, .. }), TyKind::TraitAscription(hir_bounds)) => {
            let bounds = tcx.explicit_item_bounds(def_id).skip_binder();
            // Assumes that the order of the traits are as written and the generic args as well
            let trait_bounds_args = bounds
                .iter()
                .filter_map(move |bound| {
                    if let Some(TraitPredicate {
                        trait_ref: TraitRef { def_id, args, .. },
                        ..
                    }) = bound.0.as_trait_clause().map(|binder| binder.skip_binder())
                    {
                        // If predicates are ever checked, this part could use some love.
                        Some(
                            tcx.generics_of(def_id)
                                .own_args_no_defaults(tcx, args)
                                .iter()
                                .filter_map(|arg| arg.as_type()),
                        )
                    } else {
                        None
                    }
                })
                .flatten()
                .zip(
                    hir_bounds
                        .iter()
                        .filter_map(|bound| bound.trait_ref())
                        .filter_map(trait_ref_args)
                        .flatten(),
                );

            let ident_index = bounds
                .iter()
                .filter_map(|(clause, _)| clause.as_projection_clause())
                .filter_map(|predicate| {
                    if let Some(ty) = predicate.term().skip_binder().as_type() {
                        // println!("AliasTerm term: {:?}", ty,);
                        Some((
                            tcx.item_ident(predicate.skip_binder().def_id()),
                            (
                                ty,
                                predicate
                                    .skip_binder()
                                    .projection_term
                                    .own_args(tcx)
                                    .iter()
                                    .filter_map(|arg| arg.as_type()),
                            ),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            let trait_preds =
                get_constraint_types(hir_bounds.iter().filter_map(|bound| bound.trait_ref()), ident_index);
            Box::new(trait_bounds_args.chain(trait_preds))
        },

        (ty::Tuple(tys), TyKind::Tup(hir_tys)) => Box::new(tys.iter().zip(hir_tys.iter().copied())),

        _ => Box::new(iter::empty()),
    };
    let tys: Box<dyn Iterator<Item = TyPair<'tcx>>> = Box::new(
        tys.flat_map(move |(ty, hir_ty)| walk_ty_recursive(tcx, ty, hir_ty))
            .chain(iter::once((ty, hir_ty))),
    );
    tys
}

fn get_item_tys<'tcx>(tcx: TyCtxt<'tcx>, item: Item<'tcx>) -> impl IntoIterator<Item = TyPair<'tcx>> {
    let tys: Box<dyn Iterator<Item = TyPair<'tcx>>> = match item.kind {
        ItemKind::Const(_, _, ty, _) | ItemKind::TyAlias(_, _, ty) | ItemKind::Static(_, _, ty, _) => {
            Box::new(iter::once((tcx.type_of(item.owner_id).skip_binder(), *ty)))
        },
        ItemKind::Fn { sig, .. } => Box::new(get_tys_fn_sig(tcx, sig, item.owner_id).into_iter()),
        ItemKind::Enum(_, _, EnumDef { variants }) => {
            Box::new(variants.iter().flat_map(move |Variant { data: variant_data, .. }| {
                variant_data
                    .fields()
                    .iter()
                    .map(move |field| tcx.type_of(field.def_id).skip_binder())
                    .zip(variant_data.fields().iter().map(|field| *field.ty))
            }))
        },
        ItemKind::Struct(_, _, variant_data) | ItemKind::Union(_, _, variant_data) => Box::new(
            variant_data
                .fields()
                .iter()
                .map(move |field| tcx.type_of(field.def_id).skip_binder())
                .zip(variant_data.fields().iter().map(|field| *field.ty)),
        ),
        ItemKind::Trait(_, _, _, _, _, _, trait_items) => Box::new(
            trait_items
                .iter()
                .map(move |item| tcx.hir_trait_item(*item))
                .flat_map(move |trait_item| -> Option<Box<dyn Iterator<Item = TyPair<'_>>>> {
                    match trait_item.kind {
                        TraitItemKind::Fn(sig, _) => Some(Box::new(
                            get_tys_fn_sig(tcx, sig, trait_item.owner_id)
                                .into_iter()
                                .chain(get_tys_from_generics(tcx, trait_item.generics, trait_item.owner_id)),
                        )),
                        TraitItemKind::Const(ty, _) | TraitItemKind::Type(_, Some(ty)) => Some(Box::new(iter::once((
                            tcx.type_of(trait_item.owner_id).skip_binder(),
                            *ty,
                        )))),
                        _ => None,
                    }
                })
                .flatten(),
        ),
        // TODO: ItemKind::TraitAlias when it stabilizes
        ItemKind::Impl(Impl { items, self_ty, .. }) => Box::new(
            items
                .iter()
                .map(move |item| tcx.hir_impl_item(*item))
                .flat_map(move |impl_item| -> Option<Box<dyn Iterator<Item = TyPair<'_>>>> {
                    match impl_item.kind {
                        ImplItemKind::Fn(sig, _) => Some(Box::new(
                            get_tys_fn_sig(tcx, sig, impl_item.owner_id)
                                .into_iter()
                                .chain(get_tys_from_generics(tcx, impl_item.generics, impl_item.owner_id)),
                        )),
                        ImplItemKind::Const(ty, _) | ImplItemKind::Type(ty) => Some(Box::new(iter::once((
                            tcx.type_of(impl_item.owner_id).skip_binder(),
                            *ty,
                        )))),
                    }
                })
                .flatten()
                .chain(iter::once((tcx.type_of(item.owner_id).skip_binder(), *self_ty))),
        ),
        _ => Box::new(iter::empty()),
    };
    tys
}

fn check_typair<'tcx>(cx: &LateContext<'tcx>, (resolved_ty, hir_ty): TyPair<'tcx>) {
    for (resolved_ty, hir_ty) in walk_ty_recursive(cx.tcx, resolved_ty, hir_ty) {
        check_alias_args(cx, resolved_ty, hir_ty);
    }
}

// TODO: check if inside macro
#[allow(unused)]
impl<'tcx> LateLintPass<'tcx> for ExplicitDefaultArguments {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx rustc_hir::Stmt<'tcx>) {
        if stmt.span.from_expansion() {
            return;
        }
        match stmt.kind {
            StmtKind::Let(LetStmt { ty: Some(hir_ty), .. }) => {
                let ty = cx.tcx.type_of(stmt.hir_id.owner).skip_binder();
                check_typair(cx, (ty, **hir_ty));
            },
            _ => {},
        }
    }
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        println!("expr: {}", snippet(cx, expr.span, "<error>"));
        // println!("expr kind: {:?}", expr.kind);
        let ty = cx.typeck_results().expr_ty(expr);
        println!("type: {}", ty);
        // FIXME: all this TyKind::Path stuff
        if let ExprKind::Path(_) = expr.kind {
            println!("It's a path");
        }
        match ty.kind() {
            ty::Bool => println!("ty::Bool"),
            ty::Char => println!("ty::Char"),
            ty::Int(_) => println!("ty::Int"),
            ty::Uint(_) => println!("ty::Uint"),
            ty::Float(_) => println!("ty::Float"),
            ty::Adt(_, _) => println!("ty::Adt"),
            ty::Foreign(_) => println!("ty::Foreign"),
            ty::Str => println!("ty::Str"),
            ty::Array(_, _) => println!("ty::Array"),
            ty::Pat(_, _) => println!("ty::Pat"),
            ty::Slice(_) => println!("ty::Slice"),
            ty::RawPtr(_, _) => println!("ty::RawPtr"),
            ty::Ref(_, _, _) => println!("ty::Ref"),
            ty::FnDef(_, args) => println!("ty::FnDef, {:?}", args.as_slice()),
            ty::FnPtr(_, _) => println!("ty::FnPtr"),
            ty::UnsafeBinder(_) => println!("ty::UnsafeBinder"),
            ty::Dynamic(_, _, _) => println!("ty::Dynamic"),
            ty::Closure(_, _) => println!("ty::Closure"),
            ty::CoroutineClosure(_, _) => println!("ty::CoroutineClosure"),
            ty::Coroutine(_, _) => println!("ty::Coroutine"),
            ty::CoroutineWitness(_, _) => println!("ty::CoroutineWitness"),
            ty::Never => println!("ty::Never"),
            ty::Tuple(_) => println!("ty::Tuple"),
            ty::Alias(_, _) => println!("ty::Alias"),
            ty::Param(_) => println!("ty::Param"),
            ty::Bound(_, _) => println!("ty::Bound"),
            ty::Placeholder(_) => println!("ty::Placeholder"),
            ty::Infer(_) => println!("ty::Infer"),
            ty::Error(_) => println!("ty::Error"),
        }
        match expr.kind {
            ExprKind::Path(qpath) | ExprKind::Struct(&qpath, _, _) => match qpath {
                QPath::TypeRelative(ty, segment) => {
                    check_typair(cx, (cx.typeck_results().node_type(ty.hir_id), *ty));
                    for ty_pair in path_generic_args(iter::once(segment))
                        .into_iter()
                        .map(|hir_ty| (cx.typeck_results().node_type(hir_ty.hir_id), hir_ty))
                    {
                        check_typair(cx, ty_pair);
                    }
                },
                QPath::Resolved(ty, path) => {
                    if let Some(ty) = ty {
                        check_typair(cx, (cx.typeck_results().node_type(ty.hir_id), *ty));
                    }
                    for ty_pair in path_generic_args(path.segments)
                        .into_iter()
                        .map(|hir_ty| (cx.typeck_results().node_type(hir_ty.hir_id), hir_ty))
                    {
                        check_typair(cx, ty_pair);
                    }
                },
                _ => {},
            },
            ExprKind::Closure(Closure {
                fn_decl: FnDecl { inputs, output, .. },
                ..
            }) => {
                for input in *inputs {
                    check_typair(cx, (cx.typeck_results().node_type(input.hir_id), *input));
                }
                if let FnRetTy::Return(ty) = *output {
                    check_typair(cx, (cx.typeck_results().node_type(ty.hir_id), *ty));
                }
            },
            ExprKind::Cast(_, &ty)
            | ExprKind::Type(_, &ty)
            | ExprKind::Let(&LetExpr { ty: Some(&ty), .. })
            | ExprKind::UnsafeBinderCast(_, _, Some(&ty))
            | ExprKind::OffsetOf(&ty, _) => check_typair(cx, (cx.typeck_results().expr_ty(expr), ty)),
            _ => {},
        }
        println!();
    }
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx rustc_hir::Pat<'tcx>) {
        if pat.span.from_expansion() {
            return;
        }
        println!("found pat: {}", snippet(cx, pat.span, "<error>"));
        match pat.kind {
            PatKind::Struct(qpath, _, _) => {},
            PatKind::TupleStruct(qpath, _, _) => {},
            _ => {},
        }
    }
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if item.span.from_expansion() {
            return;
        }
        for ty_pair in get_item_tys(cx.tcx, *item) {
            check_typair(cx, ty_pair);
        }
        if let Some(generics) = item.kind.generics() {
            for tys in get_tys_from_generics(cx.tcx, generics, item.owner_id) {
                check_typair(cx, tys);
            }
        }
    }
}

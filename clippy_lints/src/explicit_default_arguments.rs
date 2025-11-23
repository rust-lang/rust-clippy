use std::iter;

use clippy_utils::MaybePath;
use clippy_utils::diagnostics::span_lint_hir;
use clippy_utils::source::snippet;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{
    self as hir, AmbigArg, EnumDef, FnRetTy, FnSig, GenericParam, GenericParamKind, Generics, Impl, ImplItem,
    ImplItemKind, Item, ItemKind, MutTy, OwnerId, Path, PathSegment, QPath, TraitItem, TraitItemKind, TyKind, Variant,
    WhereBoundPredicate, WherePredicateKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, AliasTy, ConstKind, GenericArg, GenericParamDef, Ty, UnevaluatedConst};

use rustc_session::declare_lint_pass;

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

// NOTE: this whole algorithm avoids using `lower_ty
fn check_alias_args<'tcx>(cx: &LateContext<'tcx>, resolved_ty: Ty<'tcx>, hir_ty: hir::Ty<'tcx>) {
    println!("resolved alias (ty::Ty): {resolved_ty}");
    println!(
        "instantiated alias (hir::Ty): {}",
        snippet(&cx.tcx, hir_ty.span, "<error>")
    );
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
        println!("aliased ty: {aliased_ty}");
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

    println!("map1: {defaults:?}");
    println!("map2: {aliased_to_alias:?}");

    // TODO: this could probably be broken up into a function
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
            println!("\tIt was there! `{}`", snippet(&cx.tcx, redundant_ty.span(), "<error>"));
        } else {
            // println!(
            //     "&ty ({ty}) == default_arg_val ({default_arg_val}) = {}",
            //     &ty == default_arg_val
            // );
            println!("\tIt was **not** there.");
        }
    }

    return;
}

type TyPair<'a> = (Ty<'a>, hir::Ty<'a>);

fn get_tys_fn_sig<'tcx>(
    cx: &LateContext<'tcx>,
    sig: FnSig<'tcx>,
    item_owner_id: OwnerId,
) -> Box<dyn Iterator<Item = TyPair<'tcx>> + 'tcx> {
    let poly_fn_sig = cx.tcx.fn_sig(item_owner_id).skip_binder();

    let output_ty = poly_fn_sig.output().skip_binder();
    let output = if let FnRetTy::Return(output_hir_ty) = sig.decl.output {
        vec![(output_ty, *output_hir_ty)]
    } else {
        Vec::new()
    };
    let inputs_ty = poly_fn_sig.inputs().skip_binder();
    let inputs_hir_tys = sig.decl.inputs;
    Box::new(
        inputs_ty
            .iter()
            .copied()
            .zip(inputs_hir_tys.iter().copied())
            .chain(output),
    )
}
// FIXME: check trait bounds in predicates because they can have generic args too
fn get_tys_generics_predicates<'tcx>(
    cx: &LateContext<'tcx>,
    generics: &Generics<'tcx>,
    item_owner_id: OwnerId,
) -> Box<dyn Iterator<Item = TyPair<'tcx>> + 'tcx> {
    // Binding for filter map
    let tcx = cx.tcx;

    let params = cx
        .tcx
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
    let predicates = cx
        .tcx
        .explicit_predicates_of(item_owner_id)
        .predicates
        .iter()
        .filter_map(|predicate| {
            predicate
                .0
                .as_trait_clause()
                .map(|clause| clause.self_ty().skip_binder())
                .or(predicate
                    .0
                    .as_type_outlives_clause()
                    .map(|clause| clause.skip_binder().0))
        })
        .zip(generics.predicates.iter().filter_map(|predicate| {
            if let WherePredicateKind::BoundPredicate(WhereBoundPredicate { bounded_ty, .. }) = predicate.kind {
                Some(**bounded_ty)
            } else {
                None
            }
        }));
    Box::new(params.chain(predicates))
}

fn walk_ty_recursive<'tcx>(
    // cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    hir_ty: hir::Ty<'tcx>,
) -> Box<dyn Iterator<Item = TyPair<'tcx>> + 'tcx> {
    let generic_arg_to_ty = |args: &rustc_hir::GenericArgs<'tcx>| -> Box<dyn Iterator<Item = hir::Ty<'tcx>>> {
        Box::new(args.args.iter().flat_map(|arg| match arg {
            rustc_hir::GenericArg::Type(ty) => Some(*ty.as_unambig_ty()),
            _ => None,
        }))
    };
    let result: Box<dyn Iterator<Item = TyPair<'tcx>>> = match (ty.kind(), hir_ty.kind) {
        // FIXME: if check_expr doesn't look at const args, this needs to change. Likely will need to change check_item
        // too
        (ty::Array(ty, _), TyKind::Array(hir_ty, _)) | (ty::Slice(ty), TyKind::Slice(hir_ty)) => {
            Box::new(iter::once((*ty, *hir_ty)))
        },
        (ty::RawPtr(ty, _), TyKind::Ptr(MutTy { ty: hir_ty, .. }))
        | (ty::Ref(_, ty, _), TyKind::Ref(_, MutTy { ty: hir_ty, .. })) => Box::new(iter::once((*ty, *hir_ty))),
        (
            ty::Adt(_, generics),
            TyKind::Path(QPath::Resolved(
                None,
                Path {
                    segments:
                        [
                            ..,
                            PathSegment {
                                args: Some(generics_hir),
                                ..
                            },
                        ],
                    ..
                },
            )),
        ) => Box::new(
            generics
                .iter()
                .flat_map(|arg| arg.as_type())
                .zip(generic_arg_to_ty(*generics_hir)),
        ),
        (
            ty::Alias(ty::Projection, AliasTy { args, .. }),
            TyKind::Path(QPath::TypeRelative(hir_ty, PathSegment { args: hir_gat_args, .. })),
        ) => {
            println!(
                "FOUND TYPE RELATIVE: `{:#?}`, gat args: `{:?}`",
                // snippet(cx, hir_ty.span, "<error>"),
                args.as_slice(),
                hir_gat_args
            );
            let hir_gat_args_iter = hir_gat_args.map_or_else(|| Box::new(iter::empty()), generic_arg_to_ty);
            Box::new(
                args.iter()
                    .flat_map(|arg| arg.as_type())
                    .zip(iter::once(*hir_ty).chain(hir_gat_args_iter)),
            )
        },
        (
            ty::Alias(ty::Projection, AliasTy { args, .. }),
            TyKind::Path(QPath::Resolved(
                Some(hir_ty),
                Path {
                    segments:
                        [
                            PathSegment {
                                args: hir_trait_args, ..
                            },
                            ..,
                            PathSegment { args: hir_gat_args, .. },
                        ],
                    ..
                },
            )),
        ) => {
            println!(
                "FOUND TYPE RELATIVE: `{:#?}`, trait args: `{:?}`, \n gat args: `{:?}`",
                args.as_slice(),
                hir_trait_args,
                hir_gat_args
            );
            let hir_trait_args_iter = hir_trait_args.map_or_else(|| Box::new(iter::empty()), generic_arg_to_ty);
            let hir_gat_args_iter = hir_gat_args.map_or_else(|| Box::new(iter::empty()), generic_arg_to_ty);
            Box::new(
                args.iter()
                    .flat_map(|arg| arg.as_type())
                    .zip(iter::once(*hir_ty).chain(hir_trait_args_iter).chain(hir_gat_args_iter)),
            )
        },
        _ => Box::new(iter::empty()),
    };
    Box::new(
        result
            .flat_map(|(ty, hir_ty)| walk_ty_recursive(ty, hir_ty))
            .chain(iter::once((ty, hir_ty))),
    )
}

#[allow(unused)]
impl<'tcx> LateLintPass<'tcx> for ExplicitDefaultArguments {
    // TODO: check expressions for turbofish, casts, constructor params and type qualified paths
    // TODO: check let statements
    // TODO: walk through types recursively, `Ty` and `hir::Ty` in lockstep. Check generic args and
    // inner types if it's a tuple or something like that
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let tys_to_check: Vec<TyPair<'_>> = {
            let mut tys = Vec::new();
            if let Some(ident) = item.kind.ident() {
                println!("\nchecking item `{ident}`");
            } else {
                println!("\nchecking item <no ident>");
            }
            let other_tys: &mut dyn Iterator<Item = TyPair<'_>> = match item.kind {
                ItemKind::Const(_, _, ty, _) | ItemKind::TyAlias(_, _, ty) => {
                    &mut iter::once((cx.tcx.type_of(item.owner_id).skip_binder(), *ty))
                },
                ItemKind::Fn { sig, .. } => &mut *get_tys_fn_sig(cx, sig, item.owner_id),
                ItemKind::Enum(_, _, EnumDef { variants }) => {
                    &mut variants.iter().flat_map(|Variant { data: variant_data, .. }| {
                        variant_data
                            .fields()
                            .iter()
                            .map(|field| cx.tcx.type_of(field.def_id).skip_binder())
                            .zip(variant_data.fields().iter().map(|field| *field.ty))
                    })
                },
                ItemKind::Struct(_, _, variant_data) | ItemKind::Union(_, _, variant_data) => &mut variant_data
                    .fields()
                    .iter()
                    .map(|field| cx.tcx.type_of(field.def_id).skip_binder())
                    .zip(variant_data.fields().iter().map(|field| *field.ty)),
                ItemKind::Trait(_, _, _, _, _, _, trait_items) => &mut trait_items
                    .iter()
                    .map(|item| cx.tcx.hir_trait_item(*item))
                    .flat_map(|trait_item| {
                        let tys: Option<Box<dyn Iterator<Item = TyPair<'_>>>> = match trait_item.kind {
                            TraitItemKind::Fn(sig, _) => {
                                Some(Box::new(get_tys_fn_sig(cx, sig, trait_item.owner_id).chain(
                                    get_tys_generics_predicates(cx, trait_item.generics, trait_item.owner_id),
                                )))
                            },
                            TraitItemKind::Const(ty, _) | TraitItemKind::Type(_, Some(ty)) => Some(Box::new(
                                iter::once((cx.tcx.type_of(trait_item.owner_id).skip_binder(), *ty)),
                            )),
                            _ => None,
                        };
                        tys
                    })
                    .flatten(),
                // TODO: ItemKind::TraitAlias when it stabilizes
                ItemKind::Impl(Impl { items, self_ty, .. }) => &mut items
                    .iter()
                    .map(|item| cx.tcx.hir_impl_item(*item))
                    .flat_map(|impl_item| {
                        let tys: Option<Box<dyn Iterator<Item = TyPair<'_>>>> = match impl_item.kind {
                            ImplItemKind::Fn(sig, _) => {
                                Some(Box::new(get_tys_fn_sig(cx, sig, impl_item.owner_id).chain(
                                    get_tys_generics_predicates(cx, impl_item.generics, impl_item.owner_id),
                                )))
                            },
                            ImplItemKind::Const(ty, _) | ImplItemKind::Type(ty) => Some(Box::new(iter::once((
                                cx.tcx.type_of(impl_item.owner_id).skip_binder(),
                                *ty,
                            )))),
                            _ => None,
                        };
                        tys
                    })
                    .flatten()
                    .chain(iter::once((cx.tcx.type_of(item.owner_id).skip_binder(), *self_ty))),
                _ => return,
            };

            if let Some(generics) = item.kind.generics() {
                tys.extend(get_tys_generics_predicates(cx, generics, item.owner_id));
            }
            tys.extend(other_tys);
            tys
        };

        for (resolved_ty, hir_ty) in tys_to_check
            .iter()
            .flat_map(|(ty, hir_ty)| walk_ty_recursive(*ty, *hir_ty))
            .collect::<Vec<_>>()
        {
            println!("CHECKING `{}`/`{}`", resolved_ty, snippet(cx, hir_ty.span, "<error>"));
            check_alias_args(cx, resolved_ty, hir_ty);
        }
    }
}

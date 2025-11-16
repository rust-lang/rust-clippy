use clippy_utils::MaybePath;
use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg, span_lint_hir};
use clippy_utils::source::snippet;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{self as hir, FnRetTy, ItemKind, Path, PathSegment, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, GenericArg, GenericParamDef, GenericParamDefKind, ParamTy, Ty, TyCtxt};
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

// TODO: walk through types recursively, this is where the walking in lockstep thing comes in

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
            generic_arg
                .as_type()
                .map(|ty| {
                    if let ty::Param(param) = ty.kind() {
                        Some((i, param))
                    } else {
                        None
                    }
                })
                .flatten()
        })
        .fold(
            (FxHashMap::default(), FxHashMap::default()),
            |(mut map1, mut map2), (i, param)| {
                if let Some(
                    alias_ty_param @ GenericParamDef {
                        kind: GenericParamDefKind::Type { has_default: true, .. },
                        ..
                    },
                ) = alias_ty_params.iter().find(|param_def| param_def.name == param.name)
                {
                    map1.insert(i, cx.tcx.type_of(alias_ty_param.def_id).skip_binder());
                    map2.insert(i, alias_ty_param.index);
                }
                (map1, map2)
            },
        )
}
// NOTE: this whole algorithm avoids using `lower_ty
fn check_alias_args<'tcx>(cx: &LateContext<'tcx>, resolved_ty: Ty<'tcx>, hir_ty: &hir::Ty<'tcx>) {
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
            // We aren't parsing a path or it doesn't have generics
            return;
        };
        let Res::Def(DefKind::TyAlias, alias_def_id) = cx.qpath_res(&qpath, hir_ty.hir_id()) else {
            // The ty doesn't refer to a type alias
            return;
        };
        // TODO: fill-in generic args and stuff, maybe not here
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
    let (map1, map2) = match_generics(cx, aliased_ty_args.as_slice(), alias_ty_params.as_slice());

    println!("map1: {map1:?}");
    println!("map2: {map2:?}");

    for (i, generic_arg) in resolved_ty_args.iter().enumerate() {
        let is_arg_default = map1.get(&i).copied() == generic_arg.as_type();
        // Was the default explicitly written, or was it there because just because it got resolved?
        if let Some(ty) = generic_arg.as_type()
            && let Some(default_arg_val) = map1.get(&i)
            && let Some(j) = map2.get(&i)
            // If something was specified and the resolved form of the type alias had the default,
            // then it is redundant
            && hir_ty_args.args.get(*j as usize).is_some()
        {
            println!(
                "&ty ({ty}) == default_arg_val ({default_arg_val}) = {}",
                &ty == default_arg_val
            );
            let redudant_ty = hir_ty_args.args.get(*j as usize).unwrap();
            span_lint_hir(
                &cx,
                EXPLICIT_DEFAULT_ARGUMENTS,
                redudant_ty.hir_id(),
                redudant_ty.span(),
                "redudant usage of default argument",
            );
            // println!(
            //     "\tIt was there! `{}`",
            //     snippet(&cx.tcx, hir_ty_args.args.get(*j as usize).unwrap().span(), "<error>")
            // );
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

#[allow(unused)]
impl<'tcx> LateLintPass<'tcx> for ExplicitDefaultArguments {
    // Also check expressions for turbofish, casts, constructor params and type qualified paths
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'tcx>) {
        let tys_to_check: Vec<(Ty<'_>, &hir::Ty<'_>)> = match item.kind {
            // ItemKind::Static(_, _, ty, _) => ty,
            // ItemKind::Const(_, _, ty, _) => ty,
            ItemKind::Fn { sig, ident, .. } => {
                println!("\nchecking func `{ident}`");
                // TODO: check inputs too
                let poly_fn_sig = cx.tcx.fn_sig(item.owner_id).skip_binder();

                let output_ty = poly_fn_sig.output().skip_binder();
                // `None` = unit type
                let FnRetTy::Return(output_hir_ty) = sig.decl.output else {
                    return;
                };
                let inputs_ty = poly_fn_sig.inputs().skip_binder();
                let inputs_hir_tys = sig.decl.inputs;
                let mut result = vec![(output_ty, output_hir_ty)];
                result.extend(inputs_ty.iter().copied().zip(inputs_hir_tys.iter()).collect::<Vec<_>>());
                result
            },
            // rustc_hir::ItemKind::Macro(ident, macro_def, macro_kinds) => todo!(),
            // rustc_hir::ItemKind::Mod(ident, _) => todo!(),
            // rustc_hir::ItemKind::ForeignMod { abi, items } => todo!(),
            // rustc_hir::ItemKind::GlobalAsm { asm, fake_body } => todo!(),
            // rustc_hir::ItemKind::TyAlias(ident, generics, ty) => todo!(),
            // rustc_hir::ItemKind::Enum(ident, generics, enum_def) => todo!(),
            ItemKind::Struct(_, _, variant_data) => variant_data
                .fields()
                .iter()
                .map(|field| cx.tcx.type_of(field.def_id).skip_binder())
                .zip(variant_data.fields().iter().map(|field| field.ty))
                .collect(),
            // rustc_hir::ItemKind::Union(ident, generics, variant_data) => todo!(),
            // rustc_hir::ItemKind::Trait(constness, is_auto, safety, ident, generics, generic_bounds, trait_item_ids)
            // => todo!(), rustc_hir::ItemKind::TraitAlias(ident, generics, generic_bounds) => todo!(),
            // NOTE: consider parent generics
            // rustc_hir::ItemKind::Impl(_) => todo!(),
            _ => return,
        };
        for (resolved_ty, hir_ty) in tys_to_check {
            check_alias_args(cx, resolved_ty, hir_ty);
        }
    }
    // fn check_path(&mut self, cx: &LateContext<'tcx>, path: &rustc_hir::Path<'tcx>, _:
    // rustc_hir::HirId) {     println!("`{}` defaults:", snippet(cx, path.span, "<error>"));
    //     if let Res::Def(DefKind::TyAlias, id) = path {
    //         for generic_param in cx.tcx.generics_of(id).own_params {
    //             if let Some(generic_arg) = generic_param.default_value(cx.tcx) {
    //                 generic_arg.skip_binder().kind
    //             }
    //             println!("\t- {}", )
    //         }
    //     }
    //     // TODO: use expect type alias
    // }
}

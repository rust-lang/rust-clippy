use std::borrow::Cow;

use clippy_utils::MaybePath;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{FnRetTy, GenericArg, ItemKind, QPath, TyKind};
use rustc_hir_analysis::lower_ty;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, EarlyBinder, GenericParamDefKind};
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

#[allow(unused)]
impl<'tcx> LateLintPass<'tcx> for ExplicitDefaultArguments {
    // Also check expressions for turbofish, casts, constructor params and type qualified paths
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'tcx>) {
        let _/*(generics, hir_ty, ty)*/  = match item.kind {
            // ItemKind::Static(_, _, ty, _) => ty,
            // ItemKind::Const(_, _, ty, _) => ty,
            ItemKind::Fn { sig, ident, .. } => {
                println!("checking func `{ident}`");
                // TODO: check inputs too
                let resolved_ty = {
                    let poly_fn_sig = cx.tcx.fn_sig(item.owner_id).skip_binder();
                    poly_fn_sig.output().skip_binder()
                };

                // `None` = unit type
                let FnRetTy::Return(instantiated_alias_hir) = sig.decl.output else {
                    return;
                };
                // TODO: check `TyKind::Path(qpath @ QPath::TypeRelative(_, path))` too
                let (aliased_ty, alias_ty_generic_params)  = if let TyKind::Path(qpath @ QPath::Resolved(_, path)) = instantiated_alias_hir.kind {
                    let Res::Def(DefKind::TyAlias, def_id) = cx.qpath_res(&qpath, instantiated_alias_hir.hir_id()) else {
                        return;
                    };
                    // TODO: fill-in generic args and stuff
                    (cx.tcx.type_of(def_id).skip_binder(), &cx.tcx.generics_of(def_id).own_params)
                } else {
                    return
                };
                let alias_ty_generic_defaults: Vec<_> =
                    alias_ty_generic_params
                        .iter()
                        .filter_map(|param| matches!(param.kind, GenericParamDefKind::Type { has_default: true, .. }).then(|| cx.tcx.type_of(param.def_id)))
                        .collect();
                println!("resolved alias (ty::Ty): {resolved_ty}");
                println!("instantiated alias (hir::Ty): {}", snippet(&cx.tcx, instantiated_alias_hir.span, "<error>"));
                // Defaults to the alias type
                println!("aliased ty: {aliased_ty}");
                let ty::Adt(_, aliased_ty_generic_args) = aliased_ty.kind() else {
                    return;
                };
                let ty::Adt(_, resolved_ty_generic_args) = resolved_ty.kind() else {
                    return;
                };

                /// Index of a resolved ty generic param to its default
                let mut map1 = FxHashMap::default();
                /// Index of a resolved ty generic param to its index in the alias type generics
                let mut map2 = FxHashMap::default();
                for (i, generic_arg) in aliased_ty_generic_args.iter().enumerate() {
                    if let Some(generic_arg_ty) = generic_arg.as_type()
                        && let ty::Param(param) = generic_arg_ty.kind()
                        && let Some(param_def) = alias_ty_generic_params.iter().find(|param_def| param_def.name == param.name)
                        // Does it have a default defined in the type alias?
                        && let GenericParamDefKind::Type { has_default: true, .. } = param_def.kind {
                        // Was the default explicitly written, or was it there because just because it got resolved?
                            map1.insert(i, cx.tcx.type_of(param_def.def_id).skip_binder());
                            map2.insert(i, param_def.index);
                    }
                }
                println!("map1: {map1:?}");
                println!("map2: {map2:?}");

                for (i, generic_arg) in resolved_ty_generic_args.iter().enumerate() {
                    if let Some(ty) = generic_arg.as_type()
                        && let Some(default_arg_val) = map1.get(&i)
                        {
                            println!("&ty ({ty}) == default_arg_val ({default_arg_val}) = {}", &ty == default_arg_val);
                            if let TyKind::Path(QPath::Resolved(_, path)) = instantiated_alias_hir.kind
                                && let Some(last_seg) = path.segments.last()
                                && let Some(generic_args) = last_seg.args
                                && let Some(i) = map2.get(&i)
                                // If something was specified and the resolved form of the type alias had the default,
                                // then it is redundant
                                && generic_args.args.get(*i as usize).is_some() {
                            println!("\tIt was there!");
                            // generic_args.args.iter().position(|arg| arg)
                        } else {
                            println!("\tIt was **not** there.");
                        }

                    }
                }
                // println!("alias type generics: {:#?}", alias_ty_generic_params);
                println!("alias type generic param defaults: {alias_ty_generic_defaults:?}");
                println!();
            },
            // rustc_hir::ItemKind::Macro(ident, macro_def, macro_kinds) => todo!(),
            // rustc_hir::ItemKind::Mod(ident, _) => todo!(),
            // rustc_hir::ItemKind::ForeignMod { abi, items } => todo!(),
            // rustc_hir::ItemKind::GlobalAsm { asm, fake_body } => todo!(),
            // rustc_hir::ItemKind::TyAlias(ident, generics, ty) => todo!(),
            // rustc_hir::ItemKind::Enum(ident, generics, enum_def) => todo!(),
            // rustc_hir::ItemKind::Struct(ident, generics, variant_data) => todo!(),
            // rustc_hir::ItemKind::Union(ident, generics, variant_data) => todo!(),
            // rustc_hir::ItemKind::Trait(constness, is_auto, safety, ident, generics, generic_bounds, trait_item_ids)
            // => todo!(), rustc_hir::ItemKind::TraitAlias(ident, generics, generic_bounds) => todo!(),
            // NOTE: consider parent generics
            // rustc_hir::ItemKind::Impl(_) => todo!(),
            _ => return,
        };
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

//! Tries to determine whether there are "of interest" methods that are not mentioned in
//! [`super::WATCHED_INHERENT_FUNCTIONS`] or [`super::IGNORED_INHERENT_FUNCTIONS`].
//!
//! For example, [rust-lang/rust#115443] added [`std::ffi::OsStr::as_encoded_bytes`]. Once this
//! lint's toolchain was upgraded to one that included that PR, this test failed. The function was
//! then added to [`super::WATCHED_INHERENT_FUNCTIONS`] and the test passed again.
//!
//! When Clippy is built in debug mode, this test is run after each crate has been linted. See the
//! implementation of `check_crate_post` for [`super::NeedlessConversionForTrait`].
//!
//! [rust-lang/rust#115443]: https://github.com/rust-lang/rust/pull/115443

use super::{IGNORED_INHERENT_FUNCTIONS, NeedlessConversionForTrait, WATCHED_INHERENT_FUNCTIONS};
use clippy_config::types::{ConfPath, ToSymPath, create_conf_path_map};
use clippy_utils::paths::{PathNS, lookup_path};
use clippy_utils::{is_no_core_crate, is_no_std_crate, sym};
use rustc_hir::Safety;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_lint::LateContext;
use rustc_middle::ty::fast_reject::SimplifiedType;
use rustc_middle::ty::{self, Ty, TypeFolder};
use rustc_span::Symbol;
use std::ops::Deref;

#[expect(clippy::too_many_lines)]
pub fn check_inherent_functions(cx: &LateContext<'_>, lint: &NeedlessConversionForTrait) {
    if is_no_core_crate(cx) || is_no_std_crate(cx) {
        return;
    }

    let into_iterator_def_id = cx.tcx.get_diagnostic_item(sym::IntoIterator).unwrap();
    let iterator_def_id = cx.tcx.get_diagnostic_item(sym::Iterator).unwrap();

    let watched_type_paths = type_paths(WATCHED_INHERENT_FUNCTIONS);

    // Create a `ConfPath` map of the ignored inherent functions to make them easier to check.
    let (ignored_inherent_functions, _) = create_conf_path_map(
        cx.tcx,
        IGNORED_INHERENT_FUNCTIONS,
        PathNS::Value,
        |def_kind| matches!(def_kind, DefKind::Fn | DefKind::AssocFn),
        "function",
        false,
    );

    // To be "of interest", a function must be trait-preserving, publicly visible, and not `unsafe`.
    let of_interest = |def_id| -> bool {
        if cx.tcx.visibility(def_id) != ty::Visibility::Public {
            return false;
        }

        let assoc_item = cx.tcx.associated_item(def_id);
        if !matches!(assoc_item.kind, ty::AssocKind::Fn { .. }) {
            return false;
        }

        let fn_sig = cx.tcx.fn_sig(assoc_item.def_id).skip_binder();
        if fn_sig.safety() == Safety::Unsafe || fn_sig.skip_binder().inputs().len() != 1 {
            return false;
        }

        let input_ty = fn_sig.input(0).skip_binder();
        let output_ty = fn_sig.output().skip_binder();

        if let Some(input_item_ty) = implements_trait_with_item(cx, input_ty, into_iterator_def_id) {
            // `Option` and `Result` implement `IntoIterator`, but not `Iterator`. So, requiring the output type
            // to implement `Iterator` filters out functions that return an `Option` or `Result`.
            if let Some(output_item_ty) = implements_trait_with_item(cx, output_ty, iterator_def_id)
                && input_item_ty == output_item_ty
            {
                return true;
            }
        } else {
            // Sanity. Because of the special precautions taken below (see `replace_ty_params_with_global_ty`),
            // we should not get here with `std::vec::Vec`.
            assert!(!input_ty.to_string().starts_with("std::vec::Vec"), "{input_ty}");
        }

        [input_ty, output_ty].into_iter().all(|ty| {
            let ty = peel_unwanted(cx, def_id, ty);
            ty.is_slice()
                || ty.is_str()
                || ty.ty_adt_def().is_some_and(|adt_def| {
                    watched_type_paths
                        .iter()
                        .any(|path| lookup_path(cx.tcx, PathNS::Type, path).contains(&adt_def.did()))
                })
        })
    };

    let slice_incoherent_impl_def_ids = cx
        .tcx
        .incoherent_impls(SimplifiedType::Slice)
        .iter()
        .filter(|&impl_def_id| {
            // Filter out cases like `core::slice::ascii::<impl [u8]>::trim_ascii`.
            let ty::Slice(ty) = cx.tcx.type_of(impl_def_id).skip_binder().kind() else {
                panic!("impl is not for a slice");
            };
            matches!(ty.kind(), ty::Param(_))
        });

    let str_incoherent_impl_def_ids = cx.tcx.incoherent_impls(SimplifiedType::Str);

    let watched_type_path_impl_def_ids = watched_type_paths
        .iter()
        .flat_map(|type_path| lookup_path(cx.tcx, PathNS::Type, type_path))
        .flat_map(|def_id| cx.tcx.inherent_impls(def_id));

    let watched_impl_def_ids = slice_incoherent_impl_def_ids
        .chain(str_incoherent_impl_def_ids)
        .chain(watched_type_path_impl_def_ids)
        .copied()
        .collect::<Vec<_>>();

    // Verify that watched and ignored inherent functions are "of interest".
    let watched_and_ignored_inherent_functions = WATCHED_INHERENT_FUNCTIONS
        .iter()
        .chain(IGNORED_INHERENT_FUNCTIONS.iter())
        .filter_map(|conf_path| {
            let sym_path = conf_path.to_sym_path();

            if sym_path.first() == Some(&sym::slice) || sym_path.first() == Some(&sym::str) {
                return None;
            }

            let def_id = lookup_path(cx.tcx, PathNS::Value, &sym_path).into_iter().next();

            Some((sym_path, def_id))
        })
        .collect::<Vec<_>>();
    for (_, def_id) in &watched_and_ignored_inherent_functions {
        let &Some(def_id) = def_id else {
            panic!("`lookup_path` failed for some paths: {watched_and_ignored_inherent_functions:#?}")
        };

        assert!(of_interest(def_id), "{:?} is not of interest", cx.get_def_path(def_id));
    }

    // Verify that watched inherent functions are complete(ish).
    for impl_def_id in &watched_impl_def_ids {
        for assoc_item_def_id in cx.tcx.associated_item_def_ids(impl_def_id) {
            if of_interest(*assoc_item_def_id) {
                assert!(
                    lint.watched_inherent_functions_builtin.contains_key(assoc_item_def_id)
                        || ignored_inherent_functions.contains_key(assoc_item_def_id),
                    "{:?} is missing",
                    cx.get_def_path(*assoc_item_def_id)
                );
            }
        }
    }

    // Verify that every non-primitive, watched inherent function is associated with a `type_paths`
    // impl.
    let mut watched_inherent_functions = lint.watched_inherent_functions_builtin.clone();
    for &impl_def_id in &watched_impl_def_ids {
        for assoc_item_def_id in cx.tcx.associated_item_def_ids(impl_def_id) {
            watched_inherent_functions.remove(assoc_item_def_id);
        }
    }
    assert!(watched_inherent_functions.is_empty(), "{watched_inherent_functions:?}");
}

fn type_paths<T, const REPLACEABLE: bool>(conf_paths: &[ConfPath<T, REPLACEABLE>]) -> Vec<Vec<Symbol>>
where
    T: Deref,
    <T as Deref>::Target: ToSymPath,
{
    let mut type_paths = conf_paths
        .iter()
        .map(|conf_path| conf_path.to_sym_path().split_last().unwrap().1.to_owned())
        .collect::<Vec<_>>();

    type_paths.dedup();

    type_paths
}

// See comment preceding `replace_ty_params_with_global_ty` re type parameters. If `ty` contains any
// constant parameters, `implements_trait_with_item` returns `None`.
fn implements_trait_with_item<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, trait_id: DefId) -> Option<Ty<'tcx>> {
    if let Some(adt_def) = ty.ty_adt_def()
        && cx
            .tcx
            .generics_of(adt_def.did())
            .own_params
            .iter()
            .any(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
    {
        return None;
    }

    cx.get_associated_type(replace_ty_params_with_global_ty(cx, ty), trait_id, sym::Item)
}

// This is a hack. For `get_associated_type` to return `Some(..)`, all of its argument type's type
// parameters must be substituted for. One of the types of interest is `Vec`, and its second type
// parameter must implement `alloc::alloc::Allocator`. So we instantiate all type parameters with
// the default `Allocator`, `alloc::alloc::Global`. A more robust solution would at least consider
// trait bounds and alert when a trait other than `Allocator` was encountered.
fn replace_ty_params_with_global_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    let global_def_id = cx.tcx.lang_items().global_alloc_ty().unwrap();
    let global_adt_def = cx.tcx.adt_def(global_def_id);
    let global_ty = Ty::new_adt(cx.tcx, global_adt_def, ty::List::empty());
    ty::BottomUpFolder {
        tcx: cx.tcx,
        ty_op: |ty| {
            if matches!(ty.kind(), ty::Param(_)) {
                global_ty
            } else {
                ty
            }
        },
        lt_op: std::convert::identity,
        ct_op: std::convert::identity,
    }
    .fold_ty(ty)
}

fn peel_unwanted<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, mut ty: Ty<'tcx>) -> Ty<'tcx> {
    let owned_box = cx.tcx.lang_items().owned_box().unwrap();
    loop {
        match ty.kind() {
            ty::Ref(_, referent_ty, _) => {
                ty = *referent_ty;
                continue;
            },
            ty::Adt(adt_def, generic_args) if adt_def.did() == owned_box => {
                ty = generic_args[0].expect_ty();
                continue;
            },
            _ => {},
        }

        if let Some(as_ref_ty) = strip_as_ref(cx, def_id, ty) {
            ty = as_ref_ty;
            continue;
        }

        break;
    }

    ty
}

fn strip_as_ref<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    cx.tcx.param_env(def_id).caller_bounds().iter().find_map(|predicate| {
        if let ty::ClauseKind::Trait(ty::TraitPredicate { trait_ref, .. }) = predicate.kind().skip_binder()
            && cx.tcx.get_diagnostic_item(sym::AsRef) == Some(trait_ref.def_id)
            && let [self_arg, generic_arg] = trait_ref.args.as_slice()
            && self_arg.kind() == ty::GenericArgKind::Type(ty)
            && let ty::GenericArgKind::Type(subst_ty) = generic_arg.kind()
        {
            Some(subst_ty)
        } else {
            None
        }
    })
}

//! Util methods for [`rustc_middle::ty`]

#![allow(clippy::module_name_repetitions)]

use rustc_ast::ast::Mutability;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, LangItem, TyKind, Unsafety};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::LateContext;
use rustc_middle::mir::interpret::{ConstValue, Scalar};
use rustc_middle::ty::subst::{GenericArg, GenericArgKind, Subst};
use rustc_middle::ty::{
    self, AdtDef, Binder, FnSig, IntTy, Predicate, PredicateKind, Ty, TyCtxt, TypeFoldable, UintTy, VariantDiscr,
};
use rustc_span::symbol::Ident;
use rustc_span::{sym, Span, Symbol, DUMMY_SP};
use rustc_target::abi::{Size, VariantIdx};
use rustc_trait_selection::infer::InferCtxtExt;
use rustc_trait_selection::traits::query::normalize::AtExt;
use std::iter;

use crate::{match_def_path, must_use_attr, path_res, paths};

// Checks if the given type implements copy.
pub fn is_copy<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.is_copy_modulo_regions(cx.tcx.at(DUMMY_SP), cx.param_env)
}

/// Checks whether a type can be partially moved.
pub fn can_partially_move_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if has_drop(cx, ty) || is_copy(cx, ty) {
        return false;
    }
    match ty.kind() {
        ty::Param(_) => false,
        ty::Adt(def, subs) => def.all_fields().any(|f| !is_copy(cx, f.ty(cx.tcx, subs))),
        _ => true,
    }
}

/// Walks into `ty` and returns `true` if any inner type is the same as `other_ty`
pub fn contains_ty(ty: Ty<'_>, other_ty: Ty<'_>) -> bool {
    ty.walk().any(|inner| match inner.unpack() {
        GenericArgKind::Type(inner_ty) => other_ty == inner_ty,
        GenericArgKind::Lifetime(_) | GenericArgKind::Const(_) => false,
    })
}

/// Walks into `ty` and returns `true` if any inner type is an instance of the given adt
/// constructor.
pub fn contains_adt_constructor(ty: Ty<'_>, adt: AdtDef<'_>) -> bool {
    ty.walk().any(|inner| match inner.unpack() {
        GenericArgKind::Type(inner_ty) => inner_ty.ty_adt_def() == Some(adt),
        GenericArgKind::Lifetime(_) | GenericArgKind::Const(_) => false,
    })
}

/// Resolves `<T as Iterator>::Item` for `T`
/// Do not invoke without first verifying that the type implements `Iterator`
pub fn get_iterator_item_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    cx.tcx
        .get_diagnostic_item(sym::Iterator)
        .and_then(|iter_did| get_associated_type(cx, ty, iter_did, "Item"))
}

/// Returns the associated type `name` for `ty` as an implementation of `trait_id`.
/// Do not invoke without first verifying that the type implements the trait.
pub fn get_associated_type<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    trait_id: DefId,
    name: &str,
) -> Option<Ty<'tcx>> {
    cx.tcx
        .associated_items(trait_id)
        .find_by_name_and_kind(cx.tcx, Ident::from_str(name), ty::AssocKind::Type, trait_id)
        .map(|assoc| {
            let proj = cx.tcx.mk_projection(assoc.def_id, cx.tcx.mk_substs_trait(ty, &[]));
            cx.tcx.normalize_erasing_regions(cx.param_env, proj)
        })
}

/// Get the diagnostic name of a type, e.g. `sym::HashMap`. To check if a type
/// implements a trait marked with a diagnostic item use [`implements_trait`].
///
/// For a further exploitation what diagnostic items are see [diagnostic items] in
/// rustc-dev-guide.
///
/// [Diagnostic Items]: https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-items.html
pub fn get_type_diagnostic_name(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Symbol> {
    match ty.kind() {
        ty::Adt(adt, _) => cx.tcx.get_diagnostic_name(adt.did()),
        _ => None,
    }
}

/// Returns true if ty has `iter` or `iter_mut` methods
pub fn has_iter_method(cx: &LateContext<'_>, probably_ref_ty: Ty<'_>) -> Option<Symbol> {
    // FIXME: instead of this hard-coded list, we should check if `<adt>::iter`
    // exists and has the desired signature. Unfortunately FnCtxt is not exported
    // so we can't use its `lookup_method` method.
    let into_iter_collections: &[Symbol] = &[
        sym::Vec,
        sym::Option,
        sym::Result,
        sym::BTreeMap,
        sym::BTreeSet,
        sym::VecDeque,
        sym::LinkedList,
        sym::BinaryHeap,
        sym::HashSet,
        sym::HashMap,
        sym::PathBuf,
        sym::Path,
        sym::Receiver,
    ];

    let ty_to_check = match probably_ref_ty.kind() {
        ty::Ref(_, ty_to_check, _) => *ty_to_check,
        _ => probably_ref_ty,
    };

    let def_id = match ty_to_check.kind() {
        ty::Array(..) => return Some(sym::array),
        ty::Slice(..) => return Some(sym::slice),
        ty::Adt(adt, _) => adt.did(),
        _ => return None,
    };

    for &name in into_iter_collections {
        if cx.tcx.is_diagnostic_item(name, def_id) {
            return Some(cx.tcx.item_name(def_id));
        }
    }
    None
}

/// Checks whether a type implements a trait.
/// The function returns false in case the type contains an inference variable.
///
/// See:
/// * [`get_trait_def_id`](super::get_trait_def_id) to get a trait [`DefId`].
/// * [Common tools for writing lints] for an example how to use this function and other options.
///
/// [Common tools for writing lints]: https://github.com/rust-lang/rust-clippy/blob/master/doc/common_tools_writing_lints.md#checking-if-a-type-implements-a-specific-trait
pub fn implements_trait<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    trait_id: DefId,
    ty_params: &[GenericArg<'tcx>],
) -> bool {
    // Clippy shouldn't have infer types
    assert!(!ty.needs_infer());

    let ty = cx.tcx.erase_regions(ty);
    if ty.has_escaping_bound_vars() {
        return false;
    }
    let ty_params = cx.tcx.mk_substs(ty_params.iter());
    cx.tcx.infer_ctxt().enter(|infcx| {
        infcx
            .type_implements_trait(trait_id, ty, ty_params, cx.param_env)
            .must_apply_modulo_regions()
    })
}

/// Checks whether this type implements `Drop`.
pub fn has_drop<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.ty_adt_def() {
        Some(def) => def.has_dtor(cx.tcx),
        None => false,
    }
}

// Returns whether the type has #[must_use] attribute
pub fn is_must_use_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => must_use_attr(cx.tcx.get_attrs(adt.did())).is_some(),
        ty::Foreign(ref did) => must_use_attr(cx.tcx.get_attrs(*did)).is_some(),
        ty::Slice(ty) | ty::Array(ty, _) | ty::RawPtr(ty::TypeAndMut { ty, .. }) | ty::Ref(_, ty, _) => {
            // for the Array case we don't need to care for the len == 0 case
            // because we don't want to lint functions returning empty arrays
            is_must_use_ty(cx, *ty)
        },
        ty::Tuple(substs) => substs.iter().any(|ty| is_must_use_ty(cx, ty)),
        ty::Opaque(ref def_id, _) => {
            for (predicate, _) in cx.tcx.explicit_item_bounds(*def_id) {
                if let ty::PredicateKind::Trait(trait_predicate) = predicate.kind().skip_binder() {
                    if must_use_attr(cx.tcx.get_attrs(trait_predicate.trait_ref.def_id)).is_some() {
                        return true;
                    }
                }
            }
            false
        },
        ty::Dynamic(binder, _) => {
            for predicate in binder.iter() {
                if let ty::ExistentialPredicate::Trait(ref trait_ref) = predicate.skip_binder() {
                    if must_use_attr(cx.tcx.get_attrs(trait_ref.def_id)).is_some() {
                        return true;
                    }
                }
            }
            false
        },
        _ => false,
    }
}

// FIXME: Per https://doc.rust-lang.org/nightly/nightly-rustc/rustc_trait_selection/infer/at/struct.At.html#method.normalize
// this function can be removed once the `normalize` method does not panic when normalization does
// not succeed
/// Checks if `Ty` is normalizable. This function is useful
/// to avoid crashes on `layout_of`.
pub fn is_normalizable<'tcx>(cx: &LateContext<'tcx>, param_env: ty::ParamEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    is_normalizable_helper(cx, param_env, ty, &mut FxHashMap::default())
}

fn is_normalizable_helper<'tcx>(
    cx: &LateContext<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    ty: Ty<'tcx>,
    cache: &mut FxHashMap<Ty<'tcx>, bool>,
) -> bool {
    if let Some(&cached_result) = cache.get(&ty) {
        return cached_result;
    }
    // prevent recursive loops, false-negative is better than endless loop leading to stack overflow
    cache.insert(ty, false);
    let result = cx.tcx.infer_ctxt().enter(|infcx| {
        let cause = rustc_middle::traits::ObligationCause::dummy();
        if infcx.at(&cause, param_env).normalize(ty).is_ok() {
            match ty.kind() {
                ty::Adt(def, substs) => def.variants().iter().all(|variant| {
                    variant
                        .fields
                        .iter()
                        .all(|field| is_normalizable_helper(cx, param_env, field.ty(cx.tcx, substs), cache))
                }),
                _ => ty.walk().all(|generic_arg| match generic_arg.unpack() {
                    GenericArgKind::Type(inner_ty) if inner_ty != ty => {
                        is_normalizable_helper(cx, param_env, inner_ty, cache)
                    },
                    _ => true, // if inner_ty == ty, we've already checked it
                }),
            }
        } else {
            false
        }
    });
    cache.insert(ty, result);
    result
}

/// Returns `true` if the given type is a non aggregate primitive (a `bool` or `char`, any
/// integer or floating-point number type). For checking aggregation of primitive types (e.g.
/// tuples and slices of primitive type) see `is_recursively_primitive_type`
pub fn is_non_aggregate_primitive_type(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_))
}

/// Returns `true` if the given type is a primitive (a `bool` or `char`, any integer or
/// floating-point number type, a `str`, or an array, slice, or tuple of those types).
pub fn is_recursively_primitive_type(ty: Ty<'_>) -> bool {
    match *ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => true,
        ty::Ref(_, inner, _) if *inner.kind() == ty::Str => true,
        ty::Array(inner_type, _) | ty::Slice(inner_type) => is_recursively_primitive_type(inner_type),
        ty::Tuple(inner_types) => inner_types.iter().all(is_recursively_primitive_type),
        _ => false,
    }
}

/// Checks if the type is a reference equals to a diagnostic item
pub fn is_type_ref_to_diagnostic_item(cx: &LateContext<'_>, ty: Ty<'_>, diag_item: Symbol) -> bool {
    match ty.kind() {
        ty::Ref(_, ref_ty, _) => match ref_ty.kind() {
            ty::Adt(adt, _) => cx.tcx.is_diagnostic_item(diag_item, adt.did()),
            _ => false,
        },
        _ => false,
    }
}

/// Checks if the type is equal to a diagnostic item. To check if a type implements a
/// trait marked with a diagnostic item use [`implements_trait`].
///
/// For a further exploitation what diagnostic items are see [diagnostic items] in
/// rustc-dev-guide.
///
/// ---
///
/// If you change the signature, remember to update the internal lint `MatchTypeOnDiagItem`
///
/// [Diagnostic Items]: https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-items.html
pub fn is_type_diagnostic_item(cx: &LateContext<'_>, ty: Ty<'_>, diag_item: Symbol) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => cx.tcx.is_diagnostic_item(diag_item, adt.did()),
        _ => false,
    }
}

/// Checks if the type is equal to a lang item.
///
/// Returns `false` if the `LangItem` is not defined.
pub fn is_type_lang_item(cx: &LateContext<'_>, ty: Ty<'_>, lang_item: hir::LangItem) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => cx
            .tcx
            .lang_items()
            .require(lang_item)
            .map_or(false, |li| li == adt.did()),
        _ => false,
    }
}

/// Return `true` if the passed `typ` is `isize` or `usize`.
pub fn is_isize_or_usize(typ: Ty<'_>) -> bool {
    matches!(typ.kind(), ty::Int(IntTy::Isize) | ty::Uint(UintTy::Usize))
}

/// Checks if type is struct, enum or union type with the given def path.
///
/// If the type is a diagnostic item, use `is_type_diagnostic_item` instead.
/// If you change the signature, remember to update the internal lint `MatchTypeOnDiagItem`
pub fn match_type(cx: &LateContext<'_>, ty: Ty<'_>, path: &[&str]) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => match_def_path(cx, adt.did(), path),
        _ => false,
    }
}

/// Checks if the drop order for a type matters. Some std types implement drop solely to
/// deallocate memory. For these types, and composites containing them, changing the drop order
/// won't result in any observable side effects.
pub fn needs_ordered_drop<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    fn needs_ordered_drop_inner<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, seen: &mut FxHashSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty) {
            return false;
        }
        if !ty.has_significant_drop(cx.tcx, cx.param_env) {
            false
        }
        // Check for std types which implement drop, but only for memory allocation.
        else if is_type_lang_item(cx, ty, LangItem::OwnedBox)
            || matches!(
                get_type_diagnostic_name(cx, ty),
                Some(sym::HashSet | sym::Rc | sym::Arc | sym::cstring_type)
            )
            || match_type(cx, ty, &paths::WEAK_RC)
            || match_type(cx, ty, &paths::WEAK_ARC)
        {
            // Check all of the generic arguments.
            if let ty::Adt(_, subs) = ty.kind() {
                subs.types().any(|ty| needs_ordered_drop_inner(cx, ty, seen))
            } else {
                true
            }
        } else if !cx
            .tcx
            .lang_items()
            .drop_trait()
            .map_or(false, |id| implements_trait(cx, ty, id, &[]))
        {
            // This type doesn't implement drop, so no side effects here.
            // Check if any component type has any.
            match ty.kind() {
                ty::Tuple(fields) => fields.iter().any(|ty| needs_ordered_drop_inner(cx, ty, seen)),
                ty::Array(ty, _) => needs_ordered_drop_inner(cx, *ty, seen),
                ty::Adt(adt, subs) => adt
                    .all_fields()
                    .map(|f| f.ty(cx.tcx, subs))
                    .any(|ty| needs_ordered_drop_inner(cx, ty, seen)),
                _ => true,
            }
        } else {
            true
        }
    }

    needs_ordered_drop_inner(cx, ty, &mut FxHashSet::default())
}

/// Peels off all references on the type. Returns the underlying type and the number of references
/// removed.
pub fn peel_mid_ty_refs(ty: Ty<'_>) -> (Ty<'_>, usize) {
    fn peel(ty: Ty<'_>, count: usize) -> (Ty<'_>, usize) {
        if let ty::Ref(_, ty, _) = ty.kind() {
            peel(*ty, count + 1)
        } else {
            (ty, count)
        }
    }
    peel(ty, 0)
}

/// Peels off all references on the type.Returns the underlying type, the number of references
/// removed, and whether the pointer is ultimately mutable or not.
pub fn peel_mid_ty_refs_is_mutable(ty: Ty<'_>) -> (Ty<'_>, usize, Mutability) {
    fn f(ty: Ty<'_>, count: usize, mutability: Mutability) -> (Ty<'_>, usize, Mutability) {
        match ty.kind() {
            ty::Ref(_, ty, Mutability::Mut) => f(*ty, count + 1, mutability),
            ty::Ref(_, ty, Mutability::Not) => f(*ty, count + 1, Mutability::Not),
            _ => (ty, count, mutability),
        }
    }
    f(ty, 0, Mutability::Mut)
}

/// Returns `true` if the given type is an `unsafe` function.
pub fn type_is_unsafe_function<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::FnDef(..) | ty::FnPtr(_) => ty.fn_sig(cx.tcx).unsafety() == Unsafety::Unsafe,
        _ => false,
    }
}

/// Returns the base type for HIR references and pointers.
pub fn walk_ptrs_hir_ty<'tcx>(ty: &'tcx hir::Ty<'tcx>) -> &'tcx hir::Ty<'tcx> {
    match ty.kind {
        TyKind::Ptr(ref mut_ty) | TyKind::Rptr(_, ref mut_ty) => walk_ptrs_hir_ty(mut_ty.ty),
        _ => ty,
    }
}

/// Returns the base type for references and raw pointers, and count reference
/// depth.
pub fn walk_ptrs_ty_depth(ty: Ty<'_>) -> (Ty<'_>, usize) {
    fn inner(ty: Ty<'_>, depth: usize) -> (Ty<'_>, usize) {
        match ty.kind() {
            ty::Ref(_, ty, _) => inner(*ty, depth + 1),
            _ => (ty, depth),
        }
    }
    inner(ty, 0)
}

/// Returns `true` if types `a` and `b` are same types having same `Const` generic args,
/// otherwise returns `false`
pub fn same_type_and_consts<'tcx>(a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
    match (&a.kind(), &b.kind()) {
        (&ty::Adt(did_a, substs_a), &ty::Adt(did_b, substs_b)) => {
            if did_a != did_b {
                return false;
            }

            substs_a
                .iter()
                .zip(substs_b.iter())
                .all(|(arg_a, arg_b)| match (arg_a.unpack(), arg_b.unpack()) {
                    (GenericArgKind::Const(inner_a), GenericArgKind::Const(inner_b)) => inner_a == inner_b,
                    (GenericArgKind::Type(type_a), GenericArgKind::Type(type_b)) => {
                        same_type_and_consts(type_a, type_b)
                    },
                    _ => true,
                })
        },
        _ => a == b,
    }
}

/// Checks if a given type looks safe to be uninitialized.
pub fn is_uninit_value_valid_for_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match *ty.kind() {
        ty::Array(component, _) => is_uninit_value_valid_for_ty(cx, component),
        ty::Tuple(types) => types.iter().all(|ty| is_uninit_value_valid_for_ty(cx, ty)),
        ty::Adt(adt, _) => cx.tcx.lang_items().maybe_uninit() == Some(adt.did()),
        _ => false,
    }
}

/// Gets an iterator over all predicates which apply to the given item.
pub fn all_predicates_of(tcx: TyCtxt<'_>, id: DefId) -> impl Iterator<Item = &(Predicate<'_>, Span)> {
    let mut next_id = Some(id);
    iter::from_fn(move || {
        next_id.take().map(|id| {
            let preds = tcx.predicates_of(id);
            next_id = preds.parent;
            preds.predicates.iter()
        })
    })
    .flatten()
}

/// A signature for a function like type.
#[derive(Clone, Copy)]
pub enum ExprFnSig<'tcx> {
    Sig(Binder<'tcx, FnSig<'tcx>>),
    Closure(Binder<'tcx, FnSig<'tcx>>),
    Trait(Binder<'tcx, Ty<'tcx>>, Option<Binder<'tcx, Ty<'tcx>>>),
}
impl<'tcx> ExprFnSig<'tcx> {
    /// Gets the argument type at the given offset.
    pub fn input(self, i: usize) -> Binder<'tcx, Ty<'tcx>> {
        match self {
            Self::Sig(sig) => sig.input(i),
            Self::Closure(sig) => sig.input(0).map_bound(|ty| ty.tuple_fields()[i]),
            Self::Trait(inputs, _) => inputs.map_bound(|ty| ty.tuple_fields()[i]),
        }
    }

    /// Gets the result type, if one could be found. Note that the result type of a trait may not be
    /// specified.
    pub fn output(self) -> Option<Binder<'tcx, Ty<'tcx>>> {
        match self {
            Self::Sig(sig) | Self::Closure(sig) => Some(sig.output()),
            Self::Trait(_, output) => output,
        }
    }
}

/// If the expression is function like, get the signature for it.
pub fn expr_sig<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>) -> Option<ExprFnSig<'tcx>> {
    if let Res::Def(DefKind::Fn | DefKind::Ctor(_, CtorKind::Fn) | DefKind::AssocFn, id) = path_res(cx, expr) {
        Some(ExprFnSig::Sig(cx.tcx.fn_sig(id)))
    } else {
        let ty = cx.typeck_results().expr_ty_adjusted(expr).peel_refs();
        match *ty.kind() {
            ty::Closure(_, subs) => Some(ExprFnSig::Closure(subs.as_closure().sig())),
            ty::FnDef(id, subs) => Some(ExprFnSig::Sig(cx.tcx.fn_sig(id).subst(cx.tcx, subs))),
            ty::FnPtr(sig) => Some(ExprFnSig::Sig(sig)),
            ty::Dynamic(bounds, _) => {
                let lang_items = cx.tcx.lang_items();
                match bounds.principal() {
                    Some(bound)
                        if Some(bound.def_id()) == lang_items.fn_trait()
                            || Some(bound.def_id()) == lang_items.fn_once_trait()
                            || Some(bound.def_id()) == lang_items.fn_mut_trait() =>
                    {
                        let output = bounds
                            .projection_bounds()
                            .find(|p| lang_items.fn_once_output().map_or(false, |id| id == p.item_def_id()))
                            .map(|p| p.map_bound(|p| p.term.ty().expect("return type was a const")));
                        Some(ExprFnSig::Trait(bound.map_bound(|b| b.substs.type_at(0)), output))
                    },
                    _ => None,
                }
            },
            ty::Param(_) | ty::Projection(..) => {
                let mut inputs = None;
                let mut output = None;
                let lang_items = cx.tcx.lang_items();

                for (pred, _) in all_predicates_of(cx.tcx, cx.typeck_results().hir_owner.to_def_id()) {
                    let mut is_input = false;
                    if let Some(ty) = pred
                        .kind()
                        .map_bound(|pred| match pred {
                            PredicateKind::Trait(p)
                                if (lang_items.fn_trait() == Some(p.def_id())
                                    || lang_items.fn_mut_trait() == Some(p.def_id())
                                    || lang_items.fn_once_trait() == Some(p.def_id()))
                                    && p.self_ty() == ty =>
                            {
                                is_input = true;
                                Some(p.trait_ref.substs.type_at(1))
                            },
                            PredicateKind::Projection(p)
                                if Some(p.projection_ty.item_def_id) == lang_items.fn_once_output()
                                    && p.projection_ty.self_ty() == ty =>
                            {
                                is_input = false;
                                p.term.ty()
                            },
                            _ => None,
                        })
                        .transpose()
                    {
                        if is_input && inputs.is_none() {
                            inputs = Some(ty);
                        } else if !is_input && output.is_none() {
                            output = Some(ty);
                        } else {
                            // Multiple different fn trait impls. Is this even allowed?
                            return None;
                        }
                    }
                }

                inputs.map(|ty| ExprFnSig::Trait(ty, output))
            },
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub enum EnumValue {
    Unsigned(u128),
    Signed(i128),
}
impl core::ops::Add<u32> for EnumValue {
    type Output = Self;
    fn add(self, n: u32) -> Self::Output {
        match self {
            Self::Unsigned(x) => Self::Unsigned(x + u128::from(n)),
            Self::Signed(x) => Self::Signed(x + i128::from(n)),
        }
    }
}

/// Attempts to read the given constant as though it were an an enum value.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub fn read_explicit_enum_value(tcx: TyCtxt<'_>, id: DefId) -> Option<EnumValue> {
    if let Ok(ConstValue::Scalar(Scalar::Int(value))) = tcx.const_eval_poly(id) {
        match tcx.type_of(id).kind() {
            ty::Int(_) => Some(EnumValue::Signed(match value.size().bytes() {
                1 => i128::from(value.assert_bits(Size::from_bytes(1)) as u8 as i8),
                2 => i128::from(value.assert_bits(Size::from_bytes(2)) as u16 as i16),
                4 => i128::from(value.assert_bits(Size::from_bytes(4)) as u32 as i32),
                8 => i128::from(value.assert_bits(Size::from_bytes(8)) as u64 as i64),
                16 => value.assert_bits(Size::from_bytes(16)) as i128,
                _ => return None,
            })),
            ty::Uint(_) => Some(EnumValue::Unsigned(match value.size().bytes() {
                1 => value.assert_bits(Size::from_bytes(1)),
                2 => value.assert_bits(Size::from_bytes(2)),
                4 => value.assert_bits(Size::from_bytes(4)),
                8 => value.assert_bits(Size::from_bytes(8)),
                16 => value.assert_bits(Size::from_bytes(16)),
                _ => return None,
            })),
            _ => None,
        }
    } else {
        None
    }
}

/// Gets the value of the given variant.
pub fn get_discriminant_value(tcx: TyCtxt<'_>, adt: AdtDef<'_>, i: VariantIdx) -> EnumValue {
    let variant = &adt.variant(i);
    match variant.discr {
        VariantDiscr::Explicit(id) => read_explicit_enum_value(tcx, id).unwrap(),
        VariantDiscr::Relative(x) => match adt.variant((i.as_usize() - x as usize).into()).discr {
            VariantDiscr::Explicit(id) => read_explicit_enum_value(tcx, id).unwrap() + x,
            VariantDiscr::Relative(_) => EnumValue::Unsigned(x.into()),
        },
    }
}

/// Check if the given type is either `core::ffi::c_void`, `std::os::raw::c_void`, or one of the
/// platform specific `libc::<platform>::c_void` types in libc.
pub fn is_c_void(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    if let ty::Adt(adt, _) = ty.kind()
        && let &[krate, .., name] = &*cx.get_def_path(adt.did())
        && let sym::libc | sym::core | sym::std = krate
        && name.as_str() == "c_void"
    {
        true
    } else {
        false
    }
}

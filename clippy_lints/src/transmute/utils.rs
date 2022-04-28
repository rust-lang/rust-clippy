use clippy_utils::last_path_segment;
use clippy_utils::source::snippet;
use if_chain::if_chain;
use rustc_hir::{Expr, GenericArg, QPath, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{cast::CastKind, Ty};
use rustc_span::DUMMY_SP;
use rustc_typeck::check::{cast::CastCheck, FnCtxt, Inherited};

/// Gets the snippet of `Bar` in `…::transmute<Foo, &Bar>`. If that snippet is
/// not available , use
/// the type's `ToString` implementation. In weird cases it could lead to types
/// with invalid `'_`
/// lifetime, but it should be rare.
pub(super) fn get_type_snippet(cx: &LateContext<'_>, path: &QPath<'_>, to_ref_ty: Ty<'_>) -> String {
    let seg = last_path_segment(path);
    if_chain! {
        if let Some(params) = seg.args;
        if !params.parenthesized;
        if let Some(to_ty) = params.args.iter().filter_map(|arg| match arg {
            GenericArg::Type(ty) => Some(ty),
            _ => None,
        }).nth(1);
        if let TyKind::Rptr(_, ref to_ty) = to_ty.kind;
        then {
            return snippet(cx, to_ty.ty.span, &to_ref_ty.to_string()).to_string();
        }
    }

    to_ref_ty.to_string()
}

// check if the component types of the transmuted collection and the result have different ABI,
// size or alignment
pub(super) fn is_layout_incompatible<'tcx>(cx: &LateContext<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
    if let Ok(from) = cx.tcx.try_normalize_erasing_regions(cx.param_env, from)
        && let Ok(to) = cx.tcx.try_normalize_erasing_regions(cx.param_env, to)
        && let Ok(from_layout) = cx.tcx.layout_of(cx.param_env.and(from))
        && let Ok(to_layout) = cx.tcx.layout_of(cx.param_env.and(to))
    {
        from_layout.size != to_layout.size || from_layout.align.abi != to_layout.align.abi
    } else {
        // no idea about layout, so don't lint
        false
    }
}

/// Check if the type conversion can be expressed as a pointer cast, instead of
/// a transmute. In certain cases, including some invalid casts from array
/// references to pointers, this may cause additional errors to be emitted and/or
/// ICE error messages. This function will panic if that occurs.
pub(super) fn can_be_expressed_as_pointer_cast<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
) -> bool {
    use CastKind::{AddrPtrCast, ArrayPtrCast, FnPtrAddrCast, FnPtrPtrCast, PtrAddrCast, PtrPtrCast};
    matches!(
        check_cast(cx, e, from_ty, to_ty),
        Some(PtrPtrCast | PtrAddrCast | AddrPtrCast | ArrayPtrCast | FnPtrPtrCast | FnPtrAddrCast)
    )
}

/// If a cast from `from_ty` to `to_ty` is valid, returns an Ok containing the kind of
/// the cast. In certain cases, including some invalid casts from array references
/// to pointers, this may cause additional errors to be emitted and/or ICE error
/// messages. This function will panic if that occurs.
fn check_cast<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> Option<CastKind> {
    let hir_id = e.hir_id;
    let local_def_id = hir_id.owner;

    Inherited::build(cx.tcx, local_def_id).enter(|inherited| {
        let fn_ctxt = FnCtxt::new(&inherited, cx.param_env, hir_id);

        // If we already have errors, we can't be sure we can pointer cast.
        assert!(
            !fn_ctxt.errors_reported_since_creation(),
            "Newly created FnCtxt contained errors"
        );

        if let Ok(check) = CastCheck::new(
            &fn_ctxt, e, from_ty, to_ty,
            // We won't show any error to the user, so we don't care what the span is here.
            DUMMY_SP, DUMMY_SP,
        ) {
            let res = check.do_check(&fn_ctxt);

            // do_check's documentation says that it might return Ok and create
            // errors in the fcx instead of returning Err in some cases. Those cases
            // should be filtered out before getting here.
            assert!(
                !fn_ctxt.errors_reported_since_creation(),
                "`fn_ctxt` contained errors after cast check!"
            );

            res.ok()
        } else {
            None
        }
    })
}

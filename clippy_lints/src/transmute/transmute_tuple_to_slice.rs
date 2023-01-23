use super::TRANSMUTE_TUPLE_TO_SLICE;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::mir::Mutability;
use rustc_middle::ty::{self, Ty, UintTy};

/// Checks for `transmute_tuple_to_slice` lint.
/// Returns `true` if it's triggered, otherwise returns `false`.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    arg: &'tcx Expr<'_>,
    const_context: bool,
    msrv: &Msrv,
) -> bool {
    if const_context && !msrv.meets(msrvs::CONST_SLICE_FROM_RAW_PARTS) {
        return false;
    }
    match (&from_ty.kind(), &to_ty.kind()) {
        (ty::Tuple(subs), ty::Ref(_, inner, slice_mutability)) if subs.len() == 2 => {
            if subs[1].kind() != &ty::Uint(UintTy::Usize) {
                return false;
            }

            let ty::Slice(slice_type) = inner.kind() else { return false; };
            let ty::RawPtr(pt) = subs[0].kind() else { return false; };

            if &pt.ty != slice_type || &pt.mutbl != slice_mutability {
                return false;
            }

            let from_parts = match slice_mutability {
                Mutability::Not => "core::slice::from_raw_parts",
                Mutability::Mut => "core::slice::from_raw_parts_mut",
            };

            let pt = format!(
                "*{} {}",
                match pt.mutbl {
                    Mutability::Not => "const",
                    Mutability::Mut => "mut",
                },
                pt.ty
            );

            span_lint_and_then(
                cx,
                TRANSMUTE_TUPLE_TO_SLICE,
                e.span,
                &format!("transmute from `({pt}, usize)` to `&[{slice_type}]`"),
                |diag| {
                    let arg = sugg::Sugg::hir(cx, arg, "..");
                    diag.span_suggestion(
                        e.span,
                        "consider using",
                        format!("{from_parts}{arg}"),
                        Applicability::Unspecified,
                    );
                },
            );
            true
        },
        _ => false,
    }
}

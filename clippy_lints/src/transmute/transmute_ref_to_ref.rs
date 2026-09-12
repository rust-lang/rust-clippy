use super::{TRANSMUTE_BYTES_TO_STR, TRANSMUTE_PTR_TO_PTR};
use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::msrvs::Msrv;
use clippy_utils::{is_in_const_context, msrvs, std_or_core, sugg};
use rustc_errors::Applicability;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

/// Checks for `transmute_bytes_to_str` and `transmute_ptr_to_ptr` lints.
/// Returns `true` if either one triggered, otherwise returns `false`.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    arg: &'tcx Expr<'_>,
    msrv: Msrv,
) -> bool {
    let arg_sugg = || sugg::Sugg::hir_with_context(cx, arg, e.span.ctxt(), "..", &mut Applicability::Unspecified);
    if let (ty::Ref(_, ty_from, from_mutbl), ty::Ref(_, ty_to, to_mutbl)) = (*from_ty.kind(), *to_ty.kind()) {
        if let ty::Slice(slice_ty) = *ty_from.kind()
            && ty_to.is_str()
            && let ty::Uint(ty::UintTy::U8) = slice_ty.kind()
            && from_mutbl == to_mutbl
            && (from_mutbl != Mutability::Mut || msrv.meets(cx, msrvs::FROM_UTF8_MUT))
            && let Some(top_crate) = std_or_core(cx)
        {
            let postfix = if from_mutbl == Mutability::Mut { "_mut" } else { "" };
            // `transmute` became available in const contexts after `from_utf8` and `from_utf8_unchecked`.
            // `from_utf8_unchecked_mut` and `from_utf8_mut` exist in both contexts only after `transmute`'s const-MSRV,
            // so are in the precondition for the lint triggering.
            //
            // Since we got here (we compiled until here), we know that we are below `transmute`'s MSRV and not const,
            // or we are above and const. Therefore, no need for `msrv.meets` exists.
            let sugg = if is_in_const_context(cx) {
                format!("{top_crate}::str::from_utf8_unchecked{postfix}({})", arg_sugg())
            } else {
                format!("{top_crate}::str::from_utf8{postfix}({}).unwrap()", arg_sugg())
            };

            span_lint_and_sugg(
                cx,
                TRANSMUTE_BYTES_TO_STR,
                e.span,
                format!("transmute from a `{from_ty}` to a `{to_ty}`"),
                "consider using",
                sugg,
                Applicability::MaybeIncorrect,
            );

            return true;
        }

        if (cx.tcx.erase_and_anonymize_regions(from_ty) != cx.tcx.erase_and_anonymize_regions(to_ty))
            && !is_in_const_context(cx)
        {
            span_lint_and_then(
                cx,
                TRANSMUTE_PTR_TO_PTR,
                e.span,
                "transmute from a reference to a reference",
                |diag| {
                    let sugg_paren = arg_sugg()
                        .as_ty(Ty::new_ptr(cx.tcx, ty_from, from_mutbl))
                        .as_ty(Ty::new_ptr(cx.tcx, ty_to, to_mutbl));
                    let sugg = if to_mutbl == Mutability::Mut {
                        sugg_paren.mut_addr_deref()
                    } else {
                        sugg_paren.addr_deref()
                    };
                    diag.span_suggestion(e.span, "try", sugg, Applicability::Unspecified);
                },
            );

            return true;
        }
    }

    false
}

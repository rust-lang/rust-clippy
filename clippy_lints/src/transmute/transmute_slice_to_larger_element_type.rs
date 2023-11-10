use super::TRANSMUTE_SLICE_TO_LARGER_ELEMENT_TYPE;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::reindent_multiline;
use clippy_utils::sugg;
use clippy_utils::ty::approx_ty_size;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use std::borrow::Cow;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    call_to_transmute: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    transmute_arg: &'tcx Expr<'_>,
) -> bool {
    if let (ty::Ref(_, ty_from, _), ty::Ref(_, ty_to, _)) = (&from_ty.kind(), &to_ty.kind()) {
        if let (&ty::Slice(ty_elem_from), &ty::Slice(ty_elem_to)) = (&ty_from.kind(), &ty_to.kind()) {
            let ty_eleme_from_size = approx_ty_size(cx, *ty_elem_from);
            let ty_elem_to_size = approx_ty_size(cx, *ty_elem_to);
            if ty_eleme_from_size < ty_elem_to_size {
                // this is UB!!
                span_lint_and_then(
                    cx,
                    TRANSMUTE_SLICE_TO_LARGER_ELEMENT_TYPE,
                    call_to_transmute.span,
                    &format!("transmute from `&[{ty_elem_from}]` to `&[{ty_elem_to}]` results in undefined behavior"),
                    |diag| {
                        let transmute_arg = sugg::Sugg::hir(cx, transmute_arg, "..");
                        // TODO: In this case, outer unsafe block is not needed anymore. It should be removed in
                        // suggestion.
                        // FIXME: this do not compile, because temporal Vec dropped at end of outer unsafe block.
                        let sugg_reallocate = format!(
                            "{transmute_arg}\
                            .iter()\
                            .map(|item| unsafe {{ std::mem::transmute(item) }})\
                            .collect::<Vec<_>>()\
                            .as_slice()"
                        );
                        let sugg_reallocate = Cow::from(sugg_reallocate);
                        let sugg_align_to = format!("({transmute_arg}).align_to::<{ty_elem_to}>().1");
                        let sugg_align_to = Cow::from(sugg_align_to);
                        diag.note("this transmute leads out-of-bounds read");
                        diag.span_suggestions(
                            call_to_transmute.span,
                            "try",
                            [
                                reindent_multiline(sugg_reallocate, true, None).to_string(),
                                // TODO: this suggestion does not check if there's prefix and postfix.
                                // NOTE: this is not what user want to do if ty_elem_to is ZST; however,
                                // this lint will not fire in such case anyway (ZSTs cannot be larger than any type).
                                reindent_multiline(sugg_align_to, true, None).to_string(),
                            ],
                            Applicability::Unspecified,
                        );
                    },
                );

                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

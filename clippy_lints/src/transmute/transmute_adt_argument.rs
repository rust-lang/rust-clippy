use super::MUTABLE_ADT_ARGUMENT_TRANSMUTE;
use clippy_utils::diagnostics::span_lint;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, GenericArgKind, Ty};

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> bool {
    from_ty
        .walk()
        .zip(to_ty.walk())
        .filter_map(|(from_ty, to_ty)| {
            if let (GenericArgKind::Type(from_ty), GenericArgKind::Type(to_ty)) = (from_ty.kind(), to_ty.kind()) {
                Some((from_ty, to_ty))
            } else {
                None
            }
        })
        .filter(|(from_ty_inner, to_ty_inner)| {
            if let (ty::Ref(_, _, from_mut), ty::Ref(_, _, to_mut)) = (from_ty_inner.kind(), to_ty_inner.kind())
                && from_mut < to_mut
            {
                span_lint(
                    cx,
                    MUTABLE_ADT_ARGUMENT_TRANSMUTE,
                    e.span,
                    format!("transmute of type argument {from_ty_inner} to {from_ty_inner}"),
                );
                true
            } else {
                false
            }
        })
        .count()
        > 0
}

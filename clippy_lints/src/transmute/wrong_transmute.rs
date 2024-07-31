use super::WRONG_TRANSMUTE;
use clippy_utils::diagnostics::span_lint;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

/// Checks for `wrong_transmute` lint.
/// Returns `true` if it's triggered, otherwise returns `false`.
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, mut from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> bool {
    if let ty::Adt(def, args) = from_ty.kind() {
        let mut fields = def.variants().iter().filter_map(|v| non_zst_inner_ty(cx, v));
        if fields.clone().count() != 1 {
            return false;
        }
        if let Some(field) = fields.next_back() {
            from_ty = field.ty(cx.tcx, args);
        }
    }
    match (&from_ty.kind(), &to_ty.kind()) {
        (ty::Uint(_) | ty::Int(_) | ty::Float(_) | ty::Char, ty::Ref(..) | ty::RawPtr(_, _)) => {
            span_lint(
                cx,
                WRONG_TRANSMUTE,
                e.span,
                format!("transmute from a `{from_ty}` to a pointer"),
            );
            true
        },
        _ => false,
    }
}

fn non_zst_inner_ty<'a>(cx: &LateContext<'_>, variant: &'a ty::VariantDef) -> Option<&'a ty::FieldDef> {
    let tcx = cx.tcx;
    let param_env = tcx.param_env(variant.def_id);
    variant.fields.iter().find(|field| {
        let field_ty = tcx.type_of(field.did).instantiate_identity();
        let is_1zst = tcx
            .layout_of(param_env.and(field_ty))
            .is_ok_and(|layout| layout.is_1zst());
        !is_1zst
    })
}

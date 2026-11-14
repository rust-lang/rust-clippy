use rustc_hir::Expr;
use rustc_lint::LateContext;

//use super::STR_PARSE_INFER_TO_I32;

// TODO: Adjust the parameters as necessary
pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    // first check we actually have inherent str.parse
    // this part is based on clippy_lints/src/methods/byte_count_to_len.rs
    if let Some(call_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.inherent_impl_of_assoc(call_id)
        && let self_ty = cx.tcx.type_of(impl_id).instantiate_identity().skip_norm_wip()
        && self_ty.is_str()
        // Now we know we have inherent str.parse
        // Next is the target actually i32
        && let result_ty = cx.typeck_results().node_args(expr.hir_id)
            && matches!(
                result_ty.type_at(0).kind(),
                rustc_middle::ty::Int(rustc_middle::ty::IntTy::I32)
            )
    {
        // It's an i32 for sure.
        // Now I want to check if the i32 was explicit
        dbg!()
    };
}

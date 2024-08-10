use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

pub fn check<'tcx>(cx: &LateContext<'tcx>, ty_into: Ty<'_>, cast_to_hir: &'tcx rustc_hir::Ty<'tcx>) {
    if let rustc_hir::TyKind::Ptr(rustc_hir::MutTy { ty, .. }) = cast_to_hir.kind
        && matches!(ty.kind, rustc_hir::TyKind::Infer)
    {
        clippy_utils::diagnostics::span_lint_and_then(
            cx,
            super::AS_POINTER_UNDERSCORE,
            cast_to_hir.span,
            "using inferred pointer cast",
            |diag| {
                diag.span_suggestion(
                    cast_to_hir.span,
                    "use explicit type",
                    ty_into,
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

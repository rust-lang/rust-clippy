use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty::{IsSuggestable, Ty};

pub fn check<'tcx>(cx: &LateContext<'tcx>, ty_into: Ty<'tcx>, cast_to_hir: &'tcx rustc_hir::Ty<'tcx>) {
    if let rustc_hir::TyKind::Ptr(rustc_hir::MutTy { ty, .. }) = cast_to_hir.kind
        && matches!(ty.kind, rustc_hir::TyKind::Infer(()))
    {
        if ty_into.is_suggestable(cx.tcx, true) {
            clippy_utils::diagnostics::span_lint_and_sugg(
                cx,
                super::AS_POINTER_UNDERSCORE,
                cast_to_hir.span,
                "using inferred pointer cast",
                "use explicit type",
                ty_into.to_string(),
                Applicability::MachineApplicable,
            );
        } else {
            clippy_utils::diagnostics::span_lint_and_help(
                cx,
                super::AS_POINTER_UNDERSCORE,
                cast_to_hir.span,
                format!("using inferred pointer cast to unsuggestable type: `{ty_into}`"),
                None,
                "use explicit type",
            );
        }
    }
}

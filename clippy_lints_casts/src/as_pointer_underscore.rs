use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of `as *const _` or `as *mut _` conversion using inferred type.
    ///
    /// ### Why restrict this?
    /// The conversion might include a dangerous cast that might go undetected due to the type being inferred.
    ///
    /// ### Example
    /// ```no_run
    /// fn as_usize<T>(t: &T) -> usize {
    ///     // BUG: `t` is already a reference, so we will here
    ///     // return a dangling pointer to a temporary value instead
    ///     &t as *const _ as usize
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn as_usize<T>(t: &T) -> usize {
    ///     t as *const T as usize
    /// }
    /// ```
    #[clippy::version = "1.85.0"]
    pub AS_POINTER_UNDERSCORE,
    restriction,
    "detects `as *mut _` and `as *const _` conversion"
}

pub fn check<'tcx>(cx: &LateContext<'tcx>, ty_into: Ty<'_>, cast_to_hir: &'tcx rustc_hir::Ty<'tcx>) {
    if let rustc_hir::TyKind::Ptr(rustc_hir::MutTy { ty, .. }) = cast_to_hir.kind
        && matches!(ty.kind, rustc_hir::TyKind::Infer(()))
    {
        clippy_utils::diagnostics::span_lint_and_sugg(
            cx,
            AS_POINTER_UNDERSCORE,
            cast_to_hir.span,
            "using inferred pointer cast",
            "use explicit type",
            ty_into.to_string(),
            Applicability::MachineApplicable,
        );
    }
}

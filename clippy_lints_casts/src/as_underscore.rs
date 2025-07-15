use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_hir::{Expr, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of `as _` conversion using inferred type.
    ///
    /// ### Why restrict this?
    /// The conversion might include lossy conversion or a dangerous cast that might go
    /// undetected due to the type being inferred.
    ///
    /// The lint is allowed by default as using `_` is less wordy than always specifying the type.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo(n: usize) {}
    /// let n: u16 = 256;
    /// foo(n as _);
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(n: usize) {}
    /// let n: u16 = 256;
    /// foo(n as usize);
    /// ```
    #[clippy::version = "1.63.0"]
    pub AS_UNDERSCORE,
    restriction,
    "detects `as _` conversion"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, ty: &'tcx Ty<'_>) {
    if matches!(ty.kind, TyKind::Infer(())) {
        span_lint_and_then(cx, AS_UNDERSCORE, expr.span, "using `as _` conversion", |diag| {
            let ty_resolved = cx.typeck_results().expr_ty(expr);
            if let ty::Error(_) = ty_resolved.kind() {
                diag.help("consider giving the type explicitly");
            } else {
                diag.span_suggestion(
                    ty.span,
                    "consider giving the type explicitly",
                    ty_resolved,
                    Applicability::MachineApplicable,
                );
            }
        });
    }
}

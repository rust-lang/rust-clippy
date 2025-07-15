use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_note;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for a known NaN float being cast to an integer
    ///
    /// ### Why is this bad?
    /// NaNs are cast into zero, so one could simply use this and make the
    /// code more readable. The lint could also hint at a programmer error.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let _ = (0.0_f32 / 0.0) as u64;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let _ = 0_u64;
    /// ```
    #[clippy::version = "1.66.0"]
    pub CAST_NAN_TO_INT,
    suspicious,
    "casting a known floating-point NaN into an integer"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, from_ty: Ty<'_>, to_ty: Ty<'_>) {
    if from_ty.is_floating_point() && to_ty.is_integral() && is_known_nan(cx, cast_expr) {
        span_lint_and_note(
            cx,
            CAST_NAN_TO_INT,
            expr.span,
            format!("casting a known NaN to {to_ty}"),
            None,
            "this always evaluates to 0",
        );
    }
}

fn is_known_nan(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    match ConstEvalCtxt::new(cx).eval(e) {
        // FIXME(f16_f128): add these types when nan checks are available on all platforms
        Some(Constant::F64(n)) => n.is_nan(),
        Some(Constant::F32(n)) => n.is_nan(),
        _ => false,
    }
}

use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `0.0 / 0.0`.
    ///
    /// ### Why is this bad?
    /// It's less readable than `f32::NAN` or `f64::NAN`.
    ///
    /// ### Example
    /// ```no_run
    /// let nan = 0.0f32 / 0.0;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let nan = f32::NAN;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ZERO_DIVIDED_BY_ZERO,
    complexity,
    "usage of `0.0 / 0.0` to obtain NaN instead of `f32::NAN` or `f64::NAN`"
}

declare_lint_pass!(ZeroDiv => [ZERO_DIVIDED_BY_ZERO]);

impl<'tcx> LateLintPass<'tcx> for ZeroDiv {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // check for instances of 0.0/0.0
        if let ExprKind::Binary(ref op, left, right) = expr.kind
            && op.node == BinOpKind::Div
            && let ecx = ConstEvalCtxt::new(cx)
            && let ctxt = expr.span.ctxt()
            && let Some(lhs_value) = ecx.eval_local(left, ctxt)
            && let Some(rhs_value) = ecx.eval_local(right, ctxt)
            && is_float_zero(&lhs_value)
            && is_float_zero(&rhs_value)
        {
            let float_type = cx.typeck_results().expr_ty(expr);
            // Rust leaves the sign and payload of a NaN produced by arithmetic unspecified, and
            // `NAN` is one of the permitted results, so the replacement preserves semantics. Inside
            // a macro the expression may be instantiated at other types, so don't apply it blindly.
            let applicability = if expr.span.from_expansion() {
                Applicability::MaybeIncorrect
            } else {
                Applicability::MachineApplicable
            };
            span_lint_and_sugg(
                cx,
                ZERO_DIVIDED_BY_ZERO,
                expr.span,
                "constant division of `0.0` with `0.0` will always result in NaN",
                "use the constant instead",
                format!("{float_type}::NAN"),
                applicability,
            );
        }
    }
}

/// Checks for `0.0` or `-0.0`. Avoids `Constant`'s bitwise `PartialEq`, which distinguishes
/// between the two.
fn is_float_zero(c: &Constant) -> bool {
    match *c {
        // `f16` and `f128` are stored as their bit patterns; mask off the sign bit.
        Constant::F16(bits) => bits & !(1 << 15) == 0,
        Constant::F32(f) => f == 0.0,
        Constant::F64(f) => f == 0.0,
        Constant::F128(bits) => bits & !(1 << 127) == 0,
        _ => false,
    }
}

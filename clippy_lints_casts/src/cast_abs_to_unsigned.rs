use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of the `abs()` method that cast the result to unsigned.
    ///
    /// ### Why is this bad?
    /// The `unsigned_abs()` method avoids panic when called on the MIN value.
    ///
    /// ### Example
    /// ```no_run
    /// let x: i32 = -42;
    /// let y: u32 = x.abs() as u32;
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: i32 = -42;
    /// let y: u32 = x.unsigned_abs();
    /// ```
    #[clippy::version = "1.62.0"]
    pub CAST_ABS_TO_UNSIGNED,
    suspicious,
    "casting the result of `abs()` to an unsigned integer can panic"
}

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    cast_expr: &Expr<'_>,
    cast_from: Ty<'_>,
    cast_to: Ty<'_>,
    msrv: Msrv,
) {
    if let ty::Int(from) = cast_from.kind()
        && let ty::Uint(to) = cast_to.kind()
        && let ExprKind::MethodCall(method_path, receiver, [], _) = cast_expr.kind
        && method_path.ident.name == sym::abs
        && msrv.meets(cx, msrvs::UNSIGNED_ABS)
    {
        let span = if from.bit_width() == to.bit_width() {
            expr.span
        } else {
            // if the result of `.unsigned_abs` would be a different type, keep the cast
            // e.g. `i64 -> usize`, `i16 -> u8`
            cast_expr.span
        };

        span_lint_and_sugg(
            cx,
            CAST_ABS_TO_UNSIGNED,
            span,
            format!("casting the result of `{cast_from}::abs()` to {cast_to}"),
            "replace with",
            format!("{}.unsigned_abs()", Sugg::hir(cx, receiver, "..").maybe_paren()),
            Applicability::MachineApplicable,
        );
    }
}

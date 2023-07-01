use clippy_utils::{diagnostics::span_lint_and_sugg, is_from_proc_macro, source::snippet_opt};
use rustc_ast::{LitIntType, LitKind};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

use super::INEFFICIENT_POW;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp
)]
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, recv: &Expr<'_>, arg: &Expr<'_>) {
    if let ExprKind::Lit(kind) = recv.kind
        && let LitKind::Int(val, suffix) = kind.node
        && let power_used = f32::log2(val as f32)
        // Precision loss means it's not a power of two
        && power_used == (power_used as u32) as f32
        // `0` would be `1.pow()`, which we shouldn't lint (but should be disallowed in a separate lint)
        && power_used != 0.0
        && let power_used = power_used as u32
        && !is_from_proc_macro(cx, expr)
        && let Some(arg_snippet) = snippet_opt(cx, arg.span)
    {
        let suffix = match suffix {
            LitIntType::Signed(int) => int.name_str(),
            LitIntType::Unsigned(int) => int.name_str(),
            LitIntType::Unsuffixed => "",
        };

        span_lint_and_sugg(
            cx,
            INEFFICIENT_POW,
            expr.span,
            "inefficient `.pow()`",
            "use `<<` instead",
            format!("1{suffix} << ({arg_snippet} * {power_used})"),
            Applicability::MaybeIncorrect,
        );
    }
}

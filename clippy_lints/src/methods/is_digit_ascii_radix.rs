//! Lint for `c.is_digit(10)`

use super::IS_DIGIT_ASCII_RADIX;
use clippy_utils::{
    consts::constant_full_int, consts::FullInt, diagnostics::span_lint_and_sugg, meets_msrv, msrvs,
    source::snippet_with_applicability,
};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_semver::RustcVersion;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    self_arg: &'tcx Expr<'_>,
    radix: &'tcx Expr<'_>,
    msrv: Option<RustcVersion>,
) {
    if !meets_msrv(msrv, msrvs::IS_ASCII_DIGIT) {
        return;
    }

    if !cx.typeck_results().expr_ty_adjusted(self_arg).peel_refs().is_char() {
        return;
    }

    if let Some(radix_val) = constant_full_int(cx, cx.typeck_results(), radix) {
        let (num, replacement) = match radix_val {
            FullInt::S(10) | FullInt::U(10) => (10, "is_ascii_digit"),
            FullInt::S(16) | FullInt::U(16) => (16, "is_ascii_hexdigit"),
            _ => return,
        };
        let mut applicability = Applicability::MachineApplicable;

        span_lint_and_sugg(
            cx,
            IS_DIGIT_ASCII_RADIX,
            expr.span,
            &format!("use of `char::is_digit` with literal radix of {}", num),
            "try",
            format!(
                "{}.{}()",
                snippet_with_applicability(cx, self_arg.span, "..", &mut applicability),
                replacement
            ),
            applicability,
        );
    }
}

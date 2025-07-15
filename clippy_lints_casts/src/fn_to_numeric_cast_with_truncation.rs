use crate::utils;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for casts of a function pointer to a numeric type not wide enough to
    /// store an address.
    ///
    /// ### Why is this bad?
    /// Such a cast discards some bits of the function's address. If this is intended, it would be more
    /// clearly expressed by casting to `usize` first, then casting the `usize` to the intended type (with
    /// a comment) to perform the truncation.
    ///
    /// ### Example
    /// ```no_run
    /// fn fn1() -> i16 {
    ///     1
    /// };
    /// let _ = fn1 as i32;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// // Cast to usize first, then comment with the reason for the truncation
    /// fn fn1() -> i16 {
    ///     1
    /// };
    /// let fn_ptr = fn1 as usize;
    /// let fn_ptr_truncated = fn_ptr as i32;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub FN_TO_NUMERIC_CAST_WITH_TRUNCATION,
    style,
    "casting a function pointer to a numeric type not wide enough to store the address"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, cast_from: Ty<'_>, cast_to: Ty<'_>) {
    // We only want to check casts to `ty::Uint` or `ty::Int`
    let Some(to_nbits) = utils::int_ty_to_nbits(cx.tcx, cast_to) else {
        return;
    };
    match cast_from.kind() {
        ty::FnDef(..) | ty::FnPtr(..) => {
            let mut applicability = Applicability::MaybeIncorrect;
            let from_snippet = snippet_with_applicability(cx, cast_expr.span, "x", &mut applicability);

            if to_nbits < cx.tcx.data_layout.pointer_size.bits() {
                span_lint_and_sugg(
                    cx,
                    FN_TO_NUMERIC_CAST_WITH_TRUNCATION,
                    expr.span,
                    format!("casting function pointer `{from_snippet}` to `{cast_to}`, which truncates the value"),
                    "try",
                    format!("{from_snippet} as usize"),
                    applicability,
                );
            }
        },
        _ => {},
    }
}

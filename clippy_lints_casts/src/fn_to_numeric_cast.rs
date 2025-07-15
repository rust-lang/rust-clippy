use crate::utils;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for casts of function pointers to something other than `usize`.
    ///
    /// ### Why is this bad?
    /// Casting a function pointer to anything other than `usize`/`isize` is
    /// not portable across architectures. If the target type is too small the
    /// address would be truncated, and target types larger than `usize` are
    /// unnecessary.
    ///
    /// Casting to `isize` also doesn't make sense, since addresses are never
    /// signed.
    ///
    /// ### Example
    /// ```no_run
    /// fn fun() -> i32 { 1 }
    /// let _ = fun as i64;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # fn fun() -> i32 { 1 }
    /// let _ = fun as usize;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub FN_TO_NUMERIC_CAST,
    style,
    "casting a function pointer to a numeric type other than `usize`"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, cast_from: Ty<'_>, cast_to: Ty<'_>) {
    // We only want to check casts to `ty::Uint` or `ty::Int`
    let Some(to_nbits) = utils::int_ty_to_nbits(cx.tcx, cast_to) else {
        return;
    };

    match cast_from.kind() {
        ty::FnDef(..) | ty::FnPtr(..) => {
            let mut applicability = Applicability::MaybeIncorrect;

            if to_nbits >= cx.tcx.data_layout.pointer_size.bits() && !cast_to.is_usize() {
                let from_snippet = snippet_with_applicability(cx, cast_expr.span, "x", &mut applicability);
                span_lint_and_sugg(
                    cx,
                    FN_TO_NUMERIC_CAST,
                    expr.span,
                    format!("casting function pointer `{from_snippet}` to `{cast_to}`"),
                    "try",
                    format!("{from_snippet} as usize"),
                    applicability,
                );
            }
        },
        _ => {},
    }
}

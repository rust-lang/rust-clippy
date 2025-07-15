use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for casts of a function pointer to any integer type.
    ///
    /// ### Why restrict this?
    /// Casting a function pointer to an integer can have surprising results and can occur
    /// accidentally if parentheses are omitted from a function call. If you aren't doing anything
    /// low-level with function pointers then you can opt out of casting functions to integers in
    /// order to avoid mistakes. Alternatively, you can use this lint to audit all uses of function
    /// pointer casts in your code.
    ///
    /// ### Example
    /// ```no_run
    /// // fn1 is cast as `usize`
    /// fn fn1() -> u16 {
    ///     1
    /// };
    /// let _ = fn1 as usize;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// // maybe you intended to call the function?
    /// fn fn2() -> u16 {
    ///     1
    /// };
    /// let _ = fn2() as usize;
    ///
    /// // or
    ///
    /// // maybe you intended to cast it to a function type?
    /// fn fn3() -> u16 {
    ///     1
    /// }
    /// let _ = fn3 as fn() -> u16;
    /// ```
    #[clippy::version = "1.58.0"]
    pub FN_TO_NUMERIC_CAST_ANY,
    restriction,
    "casting a function pointer to any integer type"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, cast_from: Ty<'_>, cast_to: Ty<'_>) {
    // We allow casts from any function type to any function type.
    match cast_to.kind() {
        ty::FnDef(..) | ty::FnPtr(..) => return,
        _ => { /* continue to checks */ },
    }

    if let ty::FnDef(..) | ty::FnPtr(..) = cast_from.kind() {
        let mut applicability = Applicability::MaybeIncorrect;
        let from_snippet = snippet_with_applicability(cx, cast_expr.span, "..", &mut applicability);

        span_lint_and_then(
            cx,
            FN_TO_NUMERIC_CAST_ANY,
            expr.span,
            format!("casting function pointer `{from_snippet}` to `{cast_to}`"),
            |diag| {
                diag.span_suggestion_verbose(
                    expr.span,
                    "did you mean to invoke the function?",
                    format!("{from_snippet}() as {cast_to}"),
                    applicability,
                );
            },
        );
    }
}

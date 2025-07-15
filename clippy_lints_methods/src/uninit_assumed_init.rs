use clippy_utils::diagnostics::span_lint;
use clippy_utils::is_path_diagnostic_item;
use clippy_utils::ty::is_uninit_value_valid_for_ty;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `MaybeUninit::uninit().assume_init()`.
    ///
    /// ### Why is this bad?
    /// For most types, this is undefined behavior.
    ///
    /// ### Known problems
    /// For now, we accept empty tuples and tuples / arrays
    /// of `MaybeUninit`. There may be other types that allow uninitialized
    /// data, but those are not yet rigorously defined.
    ///
    /// ### Example
    /// ```no_run
    /// // Beware the UB
    /// use std::mem::MaybeUninit;
    ///
    /// let _: usize = unsafe { MaybeUninit::uninit().assume_init() };
    /// ```
    ///
    /// Note that the following is OK:
    ///
    /// ```no_run
    /// use std::mem::MaybeUninit;
    ///
    /// let _: [MaybeUninit<bool>; 5] = unsafe {
    ///     MaybeUninit::uninit().assume_init()
    /// };
    /// ```
    #[clippy::version = "1.39.0"]
    pub UNINIT_ASSUMED_INIT,
    correctness,
    "`MaybeUninit::uninit().assume_init()`"
}

/// lint for `MaybeUninit::uninit().assume_init()` (we already have the latter)
pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) {
    if let hir::ExprKind::Call(callee, []) = recv.kind
        && is_path_diagnostic_item(cx, callee, sym::maybe_uninit_uninit)
        && !is_uninit_value_valid_for_ty(cx, cx.typeck_results().expr_ty_adjusted(expr))
    {
        span_lint(
            cx,
            UNINIT_ASSUMED_INIT,
            expr.span,
            "this call for this type may be undefined behavior",
        );
    }
}

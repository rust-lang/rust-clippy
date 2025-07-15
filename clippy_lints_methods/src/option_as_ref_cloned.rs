use super::method_call;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.as_ref().cloned()` and `.as_mut().cloned()` on `Option`s
    ///
    /// ### Why is this bad?
    /// This can be written more concisely by cloning the `Option` directly.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo(bar: &Option<Vec<u8>>) -> Option<Vec<u8>> {
    ///     bar.as_ref().cloned()
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(bar: &Option<Vec<u8>>) -> Option<Vec<u8>> {
    ///     bar.clone()
    /// }
    /// ```
    #[clippy::version = "1.77.0"]
    pub OPTION_AS_REF_CLONED,
    pedantic,
    "cloning an `Option` via `as_ref().cloned()`"
}

pub(super) fn check(cx: &LateContext<'_>, cloned_recv: &Expr<'_>, cloned_ident_span: Span) {
    if let Some((method @ (sym::as_ref | sym::as_mut), as_ref_recv, [], as_ref_ident_span, _)) =
        method_call(cloned_recv)
        && is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(as_ref_recv).peel_refs(), sym::Option)
    {
        span_lint_and_sugg(
            cx,
            OPTION_AS_REF_CLONED,
            as_ref_ident_span.to(cloned_ident_span),
            format!("cloning an `Option<_>` using `.{method}().cloned()`"),
            "this can be written more concisely by cloning the `Option<_>` directly",
            "clone".into(),
            Applicability::MachineApplicable,
        );
    }
}

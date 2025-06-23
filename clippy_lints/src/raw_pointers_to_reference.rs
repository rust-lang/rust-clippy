use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::peel_middle_ty_refs;
use clippy_utils::source::{HasSession, snippet};
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks if a `&raw` pointer is created to a reference of a value, instead of a pointer to the value itself.
    ///
    /// ### Why is this bad?
    ///
    /// When creating a raw pointer to a value, the intention is to point to the value itself, rather than to the
    /// reference to the value. In other words, usally a pointer is desired, rather than a pointer to a
    /// pointer/reference.
    ///
    /// ### Example
    /// ```rust
    /// # struct S;
    /// fn foo(s: &S) -> usize {
    ///    (&raw const s).addr() // This creates a raw pointer to a reference of `s`
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// # struct S;
    /// fn foo(s: &S) -> usize {
    ///    (&raw const *s).addr() // This creates a raw pointer to the value of `s`
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub RAW_POINTERS_TO_REFERENCE,
    suspicious,
    "creating a raw pointer to a reference of a value instead of a raw pointer to the value itself"
}
declare_lint_pass!(RawPointersToReference => [RAW_POINTERS_TO_REFERENCE]);

impl LateLintPass<'_> for RawPointersToReference {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::AddrOf(BorrowKind::Raw, _, inner) = expr.kind
            && let inner_ty = cx.typeck_results().expr_ty(inner)
            && inner_ty.is_ref()
        {
            let (_, ref_count) = peel_middle_ty_refs(inner_ty);
            span_lint_and_sugg(
                cx,
                RAW_POINTERS_TO_REFERENCE,
                inner.span,
                "creating a raw pointer of reference",
                "dereference the before creating the raw pointer",
                format!("{}{}", "*".repeat(ref_count), snippet(cx.sess(), inner.span, "_")),
                Applicability::MachineApplicable,
            );
        }
    }
}

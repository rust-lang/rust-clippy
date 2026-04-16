use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
declare_clippy_lint! {
    /// ### What it does
    /// Looks for cases where PartialOrd::partial_cmp is used instead of Ord::cmp, when both exist.
    ///
    /// ### Why is this bad?
    /// It removes an unnecessary panic path.  It is more concise.  It also future-proofs a case where
    /// the Ord trait implementation is removed, resulting in a compiler error, instead of silently
    /// using the same error handling.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo(a: &str, b: &str) {
    ///     a.partial_cmp(b).unwrap();
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(a: &str, b: &str) {
    ///     a.cmp(b);
    /// }
    /// ```
    #[clippy::version = "1.96.0"]
    pub NEEDLESS_PARTIAL_CMP,
    nursery,
    "checks for uses of PartialOrd trait where Ord trait would be preferable"
}

declare_lint_pass!(NeedlessPartialCmp => [NEEDLESS_PARTIAL_CMP]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPartialCmp {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        // Look for partial_cmp method calls.
        if let ExprKind::MethodCall(path, parent, _, span) = e.kind
            && path.ident.name == sym::partial_cmp
            && cx.ty_based_def(e).opt_parent(cx).is_diag_item(cx, sym::PartialOrd)
        {
            // does this type implement Ord as well as PartialOrd?
            let implements_ord = (cx.tcx.get_diagnostic_item(sym::Ord))
                .is_some_and(|id| implements_trait(cx, cx.typeck_results().expr_ty(parent), id, &[]));
            if implements_ord {
                // if so, suggest using cmp instead
                let mut app = Applicability::MaybeIncorrect;
                let snip = snippet_with_context(cx, span, e.span.ctxt(), "_", &mut app).0;
                let new_snip = snip.to_string().replace("partial_cmp", "cmp");
                span_lint_and_sugg(
                    cx,
                    NEEDLESS_PARTIAL_CMP,
                    span,
                    "partial_cmp called when cmp is implemented",
                    "consider writing (note the change in return type)",
                    new_snip,
                    app,
                );
            }
        }
    }
}

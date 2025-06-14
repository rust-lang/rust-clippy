use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions that call themselves from their body.
    /// ### Why restrict this?
    /// In Safety Critical contexts, recursive calls can lead to catastrophic crashes if they happen to overflow
    /// the stack. Recursive calls must therefore be tightly vetted.
    /// ### Notes
    ///
    /// #### Control Flow
    /// This lint only checks for the existence of recursive calls; it doesn't discriminate based on conditional
    /// control flow. Recursive calls that are considered safe should instead be vetted and documented accordingly.
    ///
    /// #### How to vet recursive calls
    /// It is recommended that this lint be used in `deny` mode, together with #![deny(clippy::allow_attributes_without_reason)](https://rust-lang.github.io/rust-clippy/master/index.html#allow_attributes_without_reason).
    ///
    /// Once that is done, recursive calls can be vetted accordingly:
    ///
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///        #[expect(
    ///            clippy::direct_recursion,
    ///            reason = "Author has audited this function and determined that its recursive call is fine."
    ///        )]
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    ///
    /// Note the use of an `expect` attribute and a `reason` to go along with it.
    ///
    /// * The `expect` attribute (instead of `allow`) ensures that the lint being allowed is enabled. This serves as a
    /// double-check of this lint being used where the author believes it is active.
    /// * The `reason` field is required by the `clippy::allow_attributes_without_reason` lint. This is very useful for ensuring
    /// that vetting work is documented.
    ///
    /// Recursive calls that are vetted to be correct should always be annotated in such a way.
    #[clippy::version = "1.89.0"]
    pub DIRECT_RECURSION,
    restriction,
    "functions shall not call themselves directly"
}
declare_lint_pass!(DirectRecursion => [DIRECT_RECURSION]);

impl<'tcx> LateLintPass<'tcx> for DirectRecursion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(call_expr, _) = &expr.kind
            && let body_def_id = cx.tcx.hir_enclosing_body_owner(call_expr.hir_id)
            && let ExprKind::Path(c_expr_path) = call_expr.kind
            && let QPath::Resolved(_lhs, path) = c_expr_path
            && let Res::Def(DefKind::Fn, fn_path_id) = path.res
            && fn_path_id == body_def_id.into()
        {
            span_lint(
                cx,
                DIRECT_RECURSION,
                expr.span,
                "this function contains a call to itself",
            );
        }
    }
}

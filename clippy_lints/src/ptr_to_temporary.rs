use clippy_utils::{consts::is_promotable, diagnostics::span_lint_and_note, is_from_proc_macro};
use rustc_hir::{BorrowKind, Expr, ExprKind, ItemKind, OwnerNode};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for raw pointers pointing to temporary values that will **not** be promoted to a
    /// constant through
    /// [constant promotion](https://doc.rust-lang.org/stable/reference/destructors.html#constant-promotion).
    ///
    /// ### Why is this bad?
    /// Usage of such a pointer can result in Undefined Behavior, as the pointer will stop pointing
    /// to valid stack memory once the temporary is dropped.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn x() -> *const i32 {
    ///     let x = 0;
    ///     &x as *const i32
    /// }
    ///
    /// let x = x();
    /// unsafe { *x }; // ⚠️
    /// ```
    #[clippy::version = "1.72.0"]
    pub PTR_TO_TEMPORARY,
    correctness,
    "disallows returning a raw pointer to a temporary value"
}
declare_lint_pass!(PtrToTemporary => [PTR_TO_TEMPORARY]);

impl<'tcx> LateLintPass<'tcx> for PtrToTemporary {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if in_external_macro(cx.sess(), expr.span) {
            return;
        }

        // Get the final return statement if this is a return statement, or don't lint
        let expr = if let ExprKind::Ret(Some(expr)) = expr.kind {
            expr
        } else if let OwnerNode::Item(parent) = cx.tcx.hir().owner(cx.tcx.hir().get_parent_item(expr.hir_id))
            && let ItemKind::Fn(_, _, body) = parent.kind
            && let block = cx.tcx.hir().body(body).value
            && let ExprKind::Block(block, _) = block.kind
            && let Some(final_block_expr) = block.expr
            && final_block_expr.hir_id == expr.hir_id
        {
            expr
        } else {
            return;
        };

        if let ExprKind::Cast(cast_expr, _) = expr.kind
            && let ExprKind::AddrOf(BorrowKind::Ref, _, e) = cast_expr.kind
            && !is_promotable(cx, e)
            && !is_from_proc_macro(cx, expr)
        {
            span_lint_and_note(
                cx,
                PTR_TO_TEMPORARY,
                expr.span,
                "returning a raw pointer to a temporary value that cannot be promoted to a constant",
                None,
                "usage of this pointer by callers will cause Undefined Behavior",
            );
        }
    }
}

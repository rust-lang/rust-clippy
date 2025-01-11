use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::ty::is_copy;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for mutable reference on a freshly copied data due to
    /// the use of a block to return an value implementing `Copy`.
    ///
    /// ### Why is this bad?
    /// Using a block will make a copy of the block result if its type
    /// implements `Copy`. This might be an indication of a failed attempt
    /// to borrow the original data instead.
    ///
    /// ### Example
    /// ```no_run
    /// # fn f(_: &mut [i32]) {}
    /// let arr = &mut [10, 20, 30];
    /// f(&mut { *arr });
    /// ```
    /// If you intend to modify `arr` in `f`, use instead:
    /// ```no_run
    /// # fn f(_: &mut [i32]) {}
    /// let arr = &mut [10, 20, 30];
    /// f(arr);
    /// ```
    #[clippy::version = "1.86.0"]
    pub COPY_THEN_BORROW_MUT,
    suspicious,
    "mutable borrow of a data which was just copied"
}

declare_lint_pass!(CopyThenBorrowMut => [COPY_THEN_BORROW_MUT]);

impl LateLintPass<'_> for CopyThenBorrowMut {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !expr.span.from_expansion()
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, sub_expr) = expr.kind
            && let ExprKind::Block(block, _) = sub_expr.kind
            && block.span.eq_ctxt(expr.span)
            && let Some(block_expr) = block.expr
            && let block_ty = cx.typeck_results().expr_ty_adjusted(block_expr)
            && is_copy(cx, block_ty)
        {
            span_lint_and_note(
                cx,
                COPY_THEN_BORROW_MUT,
                expr.span,
                "mutable borrow of a value which was just copied",
                (!block.targeted_by_break).then_some(block_expr.span),
                "the return value of the block implements `Copy` and will be copied",
            );
        }
    }
}

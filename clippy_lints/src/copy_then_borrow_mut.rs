use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_copy;
use clippy_utils::{get_enclosing_block, is_in_test, path_to_local};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::{Block, BorrowKind, Expr, ExprKind, Mutability, Node, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for mutable reference on a freshly copied variable due to
    /// the use of a block to return an value implementing `Copy`.
    ///
    /// ### Why is this bad?
    /// Using a block will make a copy of the block result if its type
    /// implements `Copy`. This might be an indication of a failed attempt
    /// to borrow a variable, or part of a variable.
    ///
    /// ### Example
    /// ```no_run
    /// # unsafe fn unsafe_func(_: &mut i32) {}
    /// let mut a = 10;
    /// let double_a_ref = &mut unsafe { // Unsafe block needed to call `unsafe_func`
    ///    unsafe_func(&mut a);
    ///    a
    /// };
    /// ```
    /// If you intend to take a reference on `a` and you need the block,
    /// create the reference inside the block instead:
    /// ```no_run
    /// # unsafe fn unsafe_func(_: &mut i32) {}
    /// let mut a = 10;
    /// let double_a_ref = unsafe { // Unsafe block needed to call `unsafe_func`
    ///    unsafe_func(&mut a);
    ///    &mut a
    /// };
    /// ```
    #[clippy::version = "1.87.0"]
    pub COPY_THEN_BORROW_MUT,
    suspicious,
    "mutable borrow of a data which was just copied"
}

pub struct CopyThenBorrowMut {
    check_in_tests: bool,
}

impl CopyThenBorrowMut {
    pub const fn new(conf: &Conf) -> Self {
        Self {
            check_in_tests: conf.check_copy_then_borrow_mut_in_tests,
        }
    }
}

impl_lint_pass!(CopyThenBorrowMut => [COPY_THEN_BORROW_MUT]);

impl LateLintPass<'_> for CopyThenBorrowMut {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !expr.span.from_expansion()
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, sub_expr) = expr.kind
            && let ExprKind::Block(block, _) = sub_expr.kind
            && !block.targeted_by_break
            && block.span.eq_ctxt(expr.span)
            && let Some(block_expr) = block.expr
            && let block_ty = cx.typeck_results().expr_ty_adjusted(block_expr)
            && is_copy(cx, block_ty)
            && is_expr_mutable_outside_block(cx, block_expr, block)
            && (self.check_in_tests || !is_in_test(cx.tcx, expr.hir_id))
        {
            span_lint_and_then(
                cx,
                COPY_THEN_BORROW_MUT,
                expr.span,
                "mutable borrow of a value which was just copied",
                |diag| {
                    diag.multipart_suggestion(
                        "try building the reference inside the block",
                        vec![
                            (expr.span.until(block.span), String::new()),
                            (block_expr.span.shrink_to_lo(), String::from("&mut ")),
                        ],
                        Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}

/// Checks if `expr` denotes a mutable variable defined outside `block`, or, recursively, a field or
/// index of such a variable.
fn is_expr_mutable_outside_block(cx: &LateContext<'_>, mut expr: &Expr<'_>, block: &Block<'_>) -> bool {
    while let ExprKind::Field(base, _) | ExprKind::Index(base, _, _) = expr.kind {
        expr = base;
    }
    if let Some(mut hir_id) = path_to_local(expr)
        && let Node::Pat(pat) = cx.tcx.hir_node(hir_id)
        && matches!(pat.kind, PatKind::Binding(BindingMode::MUT, ..))
    {
        // Scan enclosing blocks until we find `block`, or we loop or can't find
        // blocks anymore.
        loop {
            match get_enclosing_block(cx, hir_id).map(|b| b.hir_id) {
                Some(block_id) if block_id == block.hir_id => return false,
                Some(block_id) if block_id != hir_id => hir_id = block_id,
                _ => return true,
            }
        }
    }
    false
}

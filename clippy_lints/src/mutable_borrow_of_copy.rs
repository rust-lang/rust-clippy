use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::ty::is_copy;
use clippy_utils::{get_enclosing_block, is_in_test};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::{Block, BorrowKind, Expr, ExprKind, Mutability, Node, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for taking a mutable reference on a freshly copied variable due to the use of a block returning a value implementing  `Copy`.
    ///
    /// ### Why is this bad?
    /// Using a block will make a copy of the block result if its type
    /// implements `Copy`. This might be an indication of a failed attempt
    /// to borrow a variable.
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
    #[clippy::version = "1.90.0"]
    pub MUTABLE_BORROW_OF_COPY,
    suspicious,
    "mutable borrow of a data which was just copied"
}

pub struct MutableBorrowOfCopy {
    check_in_tests: bool,
}

impl MutableBorrowOfCopy {
    pub const fn new(conf: &Conf) -> Self {
        Self {
            check_in_tests: conf.check_mutable_borrow_of_copy_in_tests,
        }
    }
}

impl_lint_pass!(MutableBorrowOfCopy => [MUTABLE_BORROW_OF_COPY]);

impl LateLintPass<'_> for MutableBorrowOfCopy {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !expr.span.from_expansion()
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, sub_expr) = expr.kind
            && let ExprKind::Block(block, _) = sub_expr.kind
            && !block.targeted_by_break
            && block.span.eq_ctxt(expr.span)
            && let Some(block_expr) = block.expr
            && let block_ty = cx.typeck_results().expr_ty_adjusted(block_expr)
            && is_copy(cx, block_ty)
            && is_copied_defined_outside_block(cx, block_expr, block)
            && (self.check_in_tests || !is_in_test(cx.tcx, expr.hir_id))
        {
            span_lint_and_then(
                cx,
                MUTABLE_BORROW_OF_COPY,
                expr.span,
                "mutable borrow of a value which was just copied",
                |diag| {
                    diag.multipart_suggestion(
                        "try building the reference inside the block",
                        vec![
                            (expr.span.until(block.span), String::new()),
                            (block_expr.span.shrink_to_lo(), String::from("&mut ")),
                        ],
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }
}

/// Checks if `expr` denotes a mutable variable defined outside `block`. This peels away field
/// accesses or indexing of such a variable first.
fn is_copied_defined_outside_block(cx: &LateContext<'_>, mut expr: &Expr<'_>, block: &Block<'_>) -> bool {
    while let ExprKind::Field(base, _) | ExprKind::Index(base, _, _) = expr.kind {
        expr = base;
    }
    if let Some(mut current) = expr.res_local_id()
        && let Node::Pat(pat) = cx.tcx.hir_node(current)
        && matches!(pat.kind, PatKind::Binding(BindingMode::MUT, ..))
    {
        // Scan enclosing blocks until we find `block` (if so, the local is defined within it), or we loop
        // or can't find blocks anymore.
        loop {
            match get_enclosing_block(cx, current).map(|b| b.hir_id) {
                Some(parent) if parent == block.hir_id => return false,
                Some(parent) if parent != current => current = parent,
                _ => return true,
            }
        }
    }
    false
}

use clippy_utils::consts::is_promotable;
use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::{is_from_proc_macro, is_temporary};
use rustc_hir::{BorrowKind, Expr, ExprKind, ItemKind, OwnerNode};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for raw pointers pointing to temporary values that will **not** be promoted to a
    /// constant through
    /// [constant promotion](https://doc.rust-lang.org/stable/reference/destructors.html#constant-promotion).
    ///
    /// ### Why is this bad?
    /// Usage of such a pointer will result in Undefined Behavior, as the pointer will stop
    /// pointing to valid stack memory once the temporary is dropped.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn returning_temp() -> *const i32 {
    ///     let x = 0;
    ///     &x as *const i32
    /// }
    ///
    /// let px = returning_temp();
    /// unsafe { *px }; // ⚠️
    /// let pv = vec![].as_ptr();
    /// unsafe { *pv }; // ⚠️
    /// ```
    #[clippy::version = "1.72.0"]
    pub PTR_TO_TEMPORARY,
    // TODO: Let's make it warn-by-default for now, and change this to deny-by-default once we know
    // there are no major FPs
    suspicious,
    "disallows obtaining raw pointers to temporary values"
}
declare_lint_pass!(PtrToTemporary => [PTR_TO_TEMPORARY]);

impl<'tcx> LateLintPass<'tcx> for PtrToTemporary {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if in_external_macro(cx.sess(), expr.span) {
            return;
        }

        _ = check_for_returning_raw_ptr(cx, expr) || check_for_dangling_as_ptr(cx, expr);
    }
}

/// Check for returning raw pointers to temporaries that are not promoted to a constant.
fn check_for_returning_raw_ptr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
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
        return false;
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
            "usage of this pointer by callers will cause Undefined Behavior as the temporary will be deallocated at \
             the end of the statement, yet the pointer will continue pointing to it, resulting in a dangling pointer",
        );

        return true;
    }

    false
}

/// Check for calls to `as_ptr` or `as_mut_ptr` that will always result in a dangling pointer, under
/// the assumption of course that `as_ptr` will return a pointer to data owned by `self`, rather
/// than returning a raw pointer to new memory.
///
/// This only lints `std` types as anything else could potentially be wrong if the above assumption
/// doesn't hold (which it should for all `std` types).
///
/// We could perhaps extend this to some external crates as well, if we want.
fn check_for_dangling_as_ptr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::MethodCall(seg, recv, [], _) = expr.kind
        && (seg.ident.name == sym::as_ptr || seg.ident.name == sym!(as_mut_ptr))
        && let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && cx.tcx.fn_sig(def_id).skip_binder().output().skip_binder().is_unsafe_ptr()
        && matches!(cx.tcx.crate_name(def_id.krate), sym::core | sym::alloc | sym::std)
        // These will almost always be promoted yet have the `as_ptr` method. Ideally we would
        // check if these would be promoted but our logic considers any function call to be
        // non-promotable, but in this case it will be as it's `'static`, soo...
        && !matches!(
            cx.typeck_results().expr_ty(recv).peel_refs().kind(),
            ty::Str | ty::Array(_, _) | ty::Slice(_)
        )
        && is_temporary(cx, recv)
        && !is_from_proc_macro(cx, expr)
    {
        span_lint_and_note(
            cx,
            PTR_TO_TEMPORARY,
            expr.span,
            "calling `as_ptr` on a temporary value",
            None,
            "usage of this pointer will cause Undefined Behavior as the temporary will be deallocated at the end of \
             the statement, yet the pointer will continue pointing to it, resulting in a dangling pointer",
        );

        return true;
    }

    false
}

// TODO: Add a check here for some blocklist for methods that return a raw pointer that we should
// lint. We can't bulk-deny these because we don't know whether it's returning something owned by
// `self` (and will thus be dropped at the end of the statement) or is returning a pointer to newly
// allocated memory, like what allocators do.
/*
fn check_for_denied_ptr_method<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    true
}
*/

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eq_expr_value;
use clippy_utils::higher::{Range, RangeTy};
use clippy_utils::source::snippet;
use clippy_utils::ty::deref_chain;
use rustc_ast::{BorrowKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty;
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects usage of manual slice splitting via (&s[..k], &s[k..])
    ///
    /// ### Why is this bad?
    ///
    /// It adds a little overhead on bound checks,
    /// and there is a more readable/idiomatic way to do it.
    ///
    /// ### Example
    ///
    /// ```
    /// let v = vec![1, 2, 3, 4, 5];
    /// let k = 3;
    /// let (_left, _right) = (&v[..k], &v[k..]);
    /// ```
    ///
    /// Could be written:
    ///
    /// ```
    /// let v = vec![1, 2, 3, 4, 5];
    /// let k = 3;
    /// let (_left, _right) = v.split_at(k);
    /// ```
    #[clippy::version = "1.100.0"]
    pub MANUAL_SPLIT_AT,
    complexity,
    "manual implementation [T]::split_at(mid:usize)"
}

impl_lint_pass!(ManualSplitAt => [MANUAL_SPLIT_AT]);

pub struct ManualSplitAt;

impl<'tcx> LateLintPass<'tcx> for ManualSplitAt {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        Self::check_manual_split_used(cx, stmt);
    }
}

impl<'tcx> ManualSplitAt {
    pub(crate) fn check_manual_split_used(cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(local) = stmt.kind
            // works only if init block is present
            && let Some(init) = local.init
            && local.els.is_none()
            && local.ty.is_none()
            // take the tuple with two expressions
            && let ExprKind::Tup([expr1, expr2]) = init.kind
            && let init_span = init.span
            // both must be borrows
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner1) = expr1.kind
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner2) = expr2.kind
            // both expressions are indexing
            && let ExprKind::Index(target1, index1, _) = inner1.kind
            && let ExprKind::Index(target2, index2, _) = inner2.kind
            // both index the same value
            // without side-effect
            && eq_expr_value(cx, init.span.ctxt(), target1, target2)
            // the receiver must be able to call .split_at(_)
            && let receiver_ty = cx.typeck_results().expr_ty_adjusted(target1).peel_refs()
            && deref_chain(cx, receiver_ty).any(has_split_at_method)
            // indexes must be in a form of [..x] and [x..]
            // no start, exclusive end
            && let Some(Range { ty: RangeTy::OpsTo, start: None, end: Some(end_index_expr), .. }) =
                Range::hir(cx, index1)
            // no end, start is present
            && let Some(Range { ty: RangeTy::OpsFrom, start: Some(start_index_expr), end: None, .. }) =
                Range::hir(cx, index2)
            // the splitting expressiong must be equal and without side effects
            && eq_expr_value(cx, init.span.ctxt(), start_index_expr, end_index_expr)
        {
            let sn_target = snippet(cx, target1.span, "..");
            let sn_index = snippet(cx, start_index_expr.span, "..");
            span_lint_and_sugg(
                cx,
                MANUAL_SPLIT_AT,
                init_span,
                format!("this could be rewritten as `{sn_target}.split_at({sn_index})`"),
                "use",
                format!("{sn_target}.split_at({sn_index})"),
                Applicability::MachineApplicable,
            );
        }
    }
}

fn has_split_at_method(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Slice(_) | ty::Str | ty::Array(..))
}

use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::{Range, RangeTy};
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet;
use clippy_utils::ty::deref_chain;
use clippy_utils::{eq_expr_value, is_in_const_context, msrvs};
use rustc_ast::{BorrowKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, Stmt, StmtKind};
use rustc_lint::{impl_lint_pass, LateContext, LateLintPass};
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
    perf,
    "manual implementation [T]::split_at(mid:usize)"
}

impl_lint_pass!(ManualSplitAt => [MANUAL_SPLIT_AT]);

pub struct ManualSplitAt {
    msrv: Msrv,
}

impl ManualSplitAt {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualSplitAt {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        Self::check_manual_split_used(cx, stmt, self.msrv);
    }
}

impl<'tcx> ManualSplitAt {
    pub(crate) fn check_manual_split_used(cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>, msrv: Msrv) {
        if let StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && local.els.is_none()
            && local.ty.is_none()
            && let ExprKind::Tup([expr1, expr2]) = init.kind
            //TODO known limitation, works only for immutable references
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner1) = expr1.kind
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner2) = expr2.kind

            && let ExprKind::Index(target1, index1, _) = inner1.kind
            && let ExprKind::Index(target2, index2, _) = inner2.kind
            && eq_expr_value(cx, init.span.ctxt(), target1, target2)
            && let receiver_ty = cx.typeck_results().expr_ty_adjusted(target1).peel_refs()
            && let receiver_chain = deref_chain(cx, receiver_ty).collect::<Vec<_>>()
            && receiver_chain.iter().any(|&ty| has_split_at_method(ty))
            && let Some(Range { ty: RangeTy::OpsTo, start: None, end: Some(end_index_expr), .. }) =
            Range::hir(cx, index1)
            && let Some(Range { ty: RangeTy::OpsFrom, start: Some(start_index_expr), end: None, .. }) =
            Range::hir(cx, index2)
            // the splitting expressiong must be equal and without side effects
            && eq_expr_value(cx, init.span.ctxt(), start_index_expr, end_index_expr)
        {
            let good = match (is_in_const_context(cx), is_str(&receiver_chain)) {
                (true, true) => msrv.meets(cx, msrvs::STR_SPLIT_AT_CONST),
                (false, true) => msrv.meets(cx, msrvs::STR_SPLIT_AT),
                (true, false) => msrv.meets(cx, msrvs::SPLIT_AT),
                (false, false) => true,
            };
            eprintln!(
                "[dbg] current={:?} str={} const={} required_msrv and good={good}",
                msrv.current(cx),
                is_str(&receiver_chain),
                is_in_const_context(cx),
            );
            if !good {
                return;
            }
            let sn_target = snippet(cx, target1.span, "..");
            let sn_index = snippet(cx, start_index_expr.span, "..");
            span_lint_and_sugg(
                cx,
                MANUAL_SPLIT_AT,
                init.span,
                "manual implementation of `split_at`",
                "use",
                format!("{sn_target}.split_at({sn_index})"),
                Applicability::MachineApplicable,
            );
        }
    }
}

fn is_str(receiver_chain: &[Ty<'_>]) -> bool {
    receiver_chain.iter().any(|ty| matches!(ty.kind(), ty::Str))
}

fn has_split_at_method(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Slice(_) | ty::Str | ty::Array(..))
}

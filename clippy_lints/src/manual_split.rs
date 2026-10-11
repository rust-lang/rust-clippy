use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::{Range, RangeTy};
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet;
use clippy_utils::ty::deref_chain;
use clippy_utils::{eq_expr_value, msrvs};
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty;
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects usage of manual slice/string splitting via `(&s[..k], &s[k..])`.
    ///
    /// ### Why is this bad?
    ///
    /// It adds a little overhead on bound checks,
    /// and there is a more readable/idiomatic way to do it.
    ///
    /// ### Limitations
    /// This lint is false negative towards const function/method invocations,
    /// and considers them unsafe to lint, because of possible side-effects
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
    "manual implementation of `split_at`"
}

impl_lint_pass!(ManualSplitAt => [MANUAL_SPLIT_AT]);

pub struct ManualSplitAt {
    msrv: Msrv,
}

impl ManualSplitAt {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }

    fn msrv_met(&self, cx: &LateContext<'_>, receiver_chain: &[Ty<'_>]) -> bool {
        let is_str = receiver_chain.iter().any(|ty| matches!(ty.kind(), ty::Str));
        if is_str {
            self.msrv.meets(cx, msrvs::STR_SPLIT_AT)
        } else {
            true
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualSplitAt {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && let ExprKind::Tup([expr1, expr2]) = init.kind
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner1) = expr1.kind
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner2) = expr2.kind
            && let ExprKind::Index(target1, indexing_expr1, _) = inner1.kind
            && let ExprKind::Index(target2, indexing_expr2, _) = inner2.kind
            && eq_expr_value(cx, init.span.ctxt(), target1, target2)
            && let receiver_ty = cx.typeck_results().expr_ty_adjusted(target1).peel_refs()
            && let receiver_chain = deref_chain(cx, receiver_ty).collect::<Vec<_>>()
            && receiver_chain
                .iter()
                .any(|ty| matches!(ty.kind(), ty::Slice(_) | ty::Str | ty::Array(..)))
            && self.msrv_met(cx, &receiver_chain)
            && let Some(bind_holder) = IndexingExprHolder::try_bind(indexing_expr1, indexing_expr2, cx)
            && eq_expr_value(
                cx,
                init.span.ctxt(),
                bind_holder.start_index_expr(),
                bind_holder.end_index_expr(),
            )
        {
            let sgn_target = snippet(cx, target1.span, "..");
            let sgn_index = snippet(cx, bind_holder.start_index_expr().span, "..");
            let (span, sugg) = match bind_holder {
                IndexingExprHolder::Plain { .. } => (init.span, format!("{sgn_target}.split_at({sgn_index})")),
                IndexingExprHolder::Reversed { .. } => {
                    let PatKind::Tuple([first, second], _) = local.pat.kind else {
                        return;
                    };
                    let first_binding = snippet(cx, second.span, "..");
                    let second_binding = snippet(cx, first.span, "..");
                    (
                        local.pat.span.to(init.span),
                        format!("({first_binding}, {second_binding}) = {sgn_target}.split_at({sgn_index})"),
                    )
                },
            };

            span_lint_and_sugg(
                cx,
                MANUAL_SPLIT_AT,
                span,
                "manual implementation of `split_at`",
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

enum IndexingExprHolder<'a> {
    Plain { from: &'a Expr<'a>, to: &'a Expr<'a> },
    Reversed { from: &'a Expr<'a>, to: &'a Expr<'a> },
}

impl<'a> IndexingExprHolder<'a> {
    fn start_index_expr(&self) -> &'a Expr<'a> {
        match self {
            Self::Plain { from, .. } | Self::Reversed { from, .. } => from,
        }
    }

    fn end_index_expr(&self) -> &'a Expr<'a> {
        match self {
            Self::Plain { to, .. } | Self::Reversed { to, .. } => to,
        }
    }

    fn try_bind(expr1: &'a Expr<'a>, expr2: &'a Expr<'a>, cx: &LateContext<'_>) -> Option<Self> {
        match (Range::hir(cx, expr1), Range::hir(cx, expr2)) {
            (
                Some(Range {
                    ty: RangeTy::OpsTo,
                    start: None,
                    end: Some(end),
                    ..
                }),
                Some(Range {
                    ty: RangeTy::OpsFrom,
                    start: Some(start),
                    end: None,
                    ..
                }),
            ) => Some(Self::Plain { from: start, to: end }),
            (
                Some(Range {
                    ty: RangeTy::OpsFrom,
                    start: Some(start),
                    end: None,
                    ..
                }),
                Some(Range {
                    ty: RangeTy::OpsTo,
                    start: None,
                    end: Some(end),
                    ..
                }),
            ) => Some(Self::Reversed { from: start, to: end }),
            _ => None,
        }
    }
}

use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::{Range, RangeTy};
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet;
use clippy_utils::ty::deref_chain;
use clippy_utils::{eq_expr_value, msrvs};
use rustc_errors::Applicability;
use rustc_hir::{Block, BorrowKind, Expr, ExprKind, Mutability, PatKind, Stmt, StmtKind, TyKind};
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
    /// let (left, right) = (&v[..k], &v[k..]);
    /// // or
    /// let left = &v[..k];
    /// let right = &v[k..];
    /// ```
    ///
    /// Could be written as:
    ///
    /// ```
    /// let v = vec![1, 2, 3, 4, 5];
    /// let k = 3;
    /// let (left, right) = v.split_at(k);
    /// ```
    #[clippy::version = "1.100.0"]
    pub MANUAL_SPLIT_AT,
    perf,
    "manual implementation of `split_at`"
}

impl_lint_pass!(ManualSplitAt => [MANUAL_SPLIT_AT]);

const WARNING_MESSAGE: &str = "manual implementation of `split_at`";

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
    fn check_block(&mut self, cx: &LateContext<'tcx>, blk: &'tcx Block<'tcx>) {
        // requiered so we don't lint overlapping windows cases
        let mut skip = false;
        for [stmt1, stmt2] in blk.stmts.array_windows::<2>() {
            if skip {
                skip = false;
                continue;
            }
            if let StmtKind::Let(local1) = stmt1.kind
                && let StmtKind::Let(local2) = stmt2.kind
                && let Some(expr1) = local1.init
                && let Some(expr2) = local2.init
                && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner1) = expr1.kind
                && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner2) = expr2.kind
                && let ExprKind::Index(target1, indexing_expr1, _) = inner1.kind
                && let ExprKind::Index(target2, indexing_expr2, _) = inner2.kind
                && let full_span = local1.span.to(local2.span)
                && eq_expr_value(cx, full_span.ctxt(), target1, target2)
                && let receiver_ty = cx.typeck_results().expr_ty_adjusted(target1).peel_refs()
                && let receiver_chain = deref_chain(cx, receiver_ty).collect::<Vec<_>>()
                && receiver_chain
                    .iter()
                    .any(|ty| matches!(ty.kind(), ty::Slice(_) | ty::Str | ty::Array(..)))
                && self.msrv_met(cx, &receiver_chain)
                && let Some(bind_holder) = IndexingExprHolder::try_bind(indexing_expr1, indexing_expr2, cx)
                && let PatKind::Binding(_, _, ident1, _) = local1.pat.kind
                && let PatKind::Binding(_, _, ident2, _) = local2.pat.kind
                && eq_expr_value(cx, full_span.ctxt(), bind_holder.from, bind_holder.to)
            {
                let type_suggestion = match (local1.ty, local2.ty) {
                    (None, None) => String::new(),
                    (a, b) => {
                        let ty_sugg1 = a.map_or("_".into(), |it| snippet(cx, it.span, "..")).to_string();
                        let ty_sugg2 = b.map_or("_".into(), |it| snippet(cx, it.span, "..")).to_string();
                        format!(": ({ty_sugg1}, {ty_sugg2})")
                    },
                };

                let (var1, var2) = match bind_holder.order {
                    BindingOrder::Plain => (snippet(cx, ident1.span, ".."), snippet(cx, ident2.span, "..")),
                    BindingOrder::Reversed => (snippet(cx, ident2.span, ".."), snippet(cx, ident1.span, "..")),
                };
                let reciever = snippet(cx, target1.span, "..");
                let indexing_expr = snippet(cx, bind_holder.from.span, "..");

                let sugg = format!("let ({var1}, {var2}){type_suggestion} = {reciever}.split_at({indexing_expr});");

                span_lint_and_sugg(
                    cx,
                    MANUAL_SPLIT_AT,
                    full_span,
                    WARNING_MESSAGE,
                    "use",
                    sugg,
                    Applicability::MachineApplicable,
                );

                skip = true;
            }
        }
    }

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
            && eq_expr_value(cx, init.span.ctxt(), bind_holder.from, bind_holder.to)
        {
            let ty_sugg = match local.ty {
                Some(local_ty) => match local_ty.kind {
                    TyKind::Tup([first, second]) => match bind_holder.order {
                        BindingOrder::Plain => format_args!(
                            ":({}, {})",
                            snippet(cx, first.span, ".."),
                            snippet(cx, second.span, "..")
                        ),
                        BindingOrder::Reversed => format_args!(
                            ":({}, {})",
                            snippet(cx, second.span, ".."),
                            snippet(cx, first.span, "..")
                        ),
                    },
                    _ => {
                        return;
                    },
                },
                // если тип НЕ указан, его можно не проставлять
                None => format_args!(""),
            };

            let sgn_target = snippet(cx, target1.span, "..");
            let sgn_index = snippet(cx, bind_holder.from.span, "..");
            let binding_sugg = match bind_holder.order {
                BindingOrder::Plain => {
                    format_args!("{}", snippet(cx, local.pat.span, ".."))
                },
                BindingOrder::Reversed => match local.pat.kind {
                    PatKind::Tuple([first, second], _) => {
                        format_args!(
                            "({}, {})",
                            snippet(cx, second.span, ".."),
                            snippet(cx, first.span, ".."),
                        )
                    },
                    _ => {
                        // for reversed binding tuple is requiered
                        return;
                    },
                },
            };
            let span = local.span.with_hi(init.span.hi());
            let sugg = format!("let {binding_sugg}{ty_sugg} = {sgn_target}.split_at({sgn_index})");
            span_lint_and_sugg(
                cx,
                MANUAL_SPLIT_AT,
                span,
                WARNING_MESSAGE,
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

#[derive(Debug)]
enum BindingOrder {
    Plain,
    Reversed,
}

#[derive(Debug)]
struct IndexingExprHolder<'a> {
    from: &'a Expr<'a>,
    to: &'a Expr<'a>,
    order: BindingOrder,
}

impl<'a> IndexingExprHolder<'a> {
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
            ) => Some(Self {
                from: start,
                to: end,
                order: BindingOrder::Plain,
            }),
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
            ) => Some(Self {
                from: start,
                to: end,
                order: BindingOrder::Reversed,
            }),
            _ => None,
        }
    }
}

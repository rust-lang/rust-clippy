use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::{
    SpanlessEq, as_some_expr, higher, is_else_clause, is_from_proc_macro, is_integer_literal, is_none_expr,
    peel_blocks, sym,
};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual implementations of `lowest_one()`, such as
    /// `if x == 0 { None } else { Some(x.trailing_zeros()) }`.
    ///
    /// ### Why is this bad?
    /// `lowest_one()` is clearer, more idiomatic, and directly conveys the intent
    /// to find the index of the least significant set bit.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 12;
    /// let index = if x == 0 { None } else { Some(x.trailing_zeros()) };
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 12;
    /// let index = x.lowest_one();
    /// ```
    #[clippy::version = "1.101.0"]
    pub MANUAL_LOWEST_ONE,
    complexity,
    "manual implementation of `lowest_one`"
}

impl_lint_pass!(ManualLowestOne => [MANUAL_LOWEST_ONE]);

pub struct ManualLowestOne {
    msrv: Msrv,
}

impl ManualLowestOne {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualLowestOne {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !expr.span.from_expansion()
            && let Some(higher::If {
                cond,
                then,
                r#else: Some(els),
            }) = higher::If::hir(expr)
            && let Some((cond_recv, is_ne)) = check_cond(cond)
            && let (none_expr, some_expr) = if is_ne { (els, then) } else { (then, els) }
            && is_none_expr(cx, peel_blocks(none_expr))
            && let Some(some_arg) = as_some_expr(cx, peel_blocks(some_expr))
            && let Some(trailing_zeros_recv) = extract_trailing_zeros_recv(some_arg)
            && !is_else_clause(cx.tcx, expr)
            && self.msrv.meets(cx, msrvs::LOWEST_ONE)
            && cx.typeck_results().expr_ty(cond_recv).is_integral()
            && SpanlessEq::new(cx).deny_side_effects().paths_by_resolution().eq_expr(
                expr.span.ctxt(),
                cond_recv,
                trailing_zeros_recv,
            )
            && !is_from_proc_macro(cx, expr)
        {
            let mut applicability = Applicability::MachineApplicable;
            let sugg = Sugg::hir_with_context(cx, cond_recv, expr.span.ctxt(), "_", &mut applicability);
            span_lint_and_sugg(
                cx,
                MANUAL_LOWEST_ONE,
                expr.span,
                "manual implementation of `lowest_one`",
                "try",
                format!("{}.lowest_one()", sugg.maybe_paren()),
                applicability,
            );
        }
    }
}

fn check_cond<'tcx>(cond: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, bool)> {
    if let ExprKind::Binary(op, lhs, rhs) = cond.kind {
        match op.node {
            BinOpKind::Eq if is_integer_literal(rhs, 0) => Some((lhs, false)),
            BinOpKind::Eq if is_integer_literal(lhs, 0) => Some((rhs, false)),
            BinOpKind::Ne if is_integer_literal(rhs, 0) => Some((lhs, true)),
            BinOpKind::Ne if is_integer_literal(lhs, 0) => Some((rhs, true)),
            _ => None,
        }
    } else {
        None
    }
}

fn extract_trailing_zeros_recv<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(method, receiver, [], _) = expr.kind
        && method.ident.name == sym::trailing_zeros
    {
        Some(receiver)
    } else {
        None
    }
}

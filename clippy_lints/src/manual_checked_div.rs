use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::{Descend, for_each_expr_without_closures};
use clippy_utils::{SpanlessEq, is_integer_literal};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Block, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Detects manual zero checks before dividing unsigned integers, such as `if x != 0 { y / x }`.
    ///
    /// ### Why is this bad?
    /// `checked_div` already handles the zero case and makes the intent clearer while avoiding a
    /// panic from a manual division.
    ///
    /// ### Example
    /// ```no_run
    /// # let (a, b) = (10u32, 5u32);
    /// if b != 0 {
    ///     let result = a / b;
    ///     println!("{result}");
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # let (a, b) = (10u32, 5u32);
    /// if let Some(result) = a.checked_div(b) {
    ///     println!("{result}");
    /// }
    /// ```
    #[clippy::version = "1.93.0"]
    pub MANUAL_CHECKED_DIV,
    nursery,
    "manual zero checks before dividing unsigned integers"
}
declare_lint_pass!(ManualCheckedDiv => [MANUAL_CHECKED_DIV]);

#[derive(Copy, Clone)]
enum NonZeroBranch {
    Then,
    Else,
}

impl LateLintPass<'_> for ManualCheckedDiv {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::If(cond, then, r#else) = expr.kind
            && let Some((divisor, branch)) = divisor_from_condition(cond)
            && is_unsigned(cx, divisor)
        {
            let Some(block) = branch_block(then, r#else, branch) else {
                return;
            };
            let mut eq = SpanlessEq::new(cx);

            for_each_expr_without_closures(block, |e| {
                if let ExprKind::Binary(binop, lhs, rhs) = e.kind
                    && binop.node == BinOpKind::Div
                    && eq.eq_expr(rhs, divisor)
                    && is_unsigned(cx, lhs)
                {
                    let mut applicability = Applicability::MaybeIncorrect;
                    let lhs_snip = Sugg::hir_with_applicability(cx, lhs, "..", &mut applicability);
                    let rhs_snip = Sugg::hir_with_applicability(cx, rhs, "..", &mut applicability);

                    span_lint_and_sugg(
                        cx,
                        MANUAL_CHECKED_DIV,
                        e.span,
                        "manual checked division",
                        "consider using `checked_div`",
                        format!("{}.checked_div({})", lhs_snip.maybe_paren(), rhs_snip),
                        applicability,
                    );

                    ControlFlow::<(), _>::Continue(Descend::No)
                } else {
                    ControlFlow::<(), _>::Continue(Descend::Yes)
                }
            });
        }
    }
}

fn divisor_from_condition<'tcx>(cond: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, NonZeroBranch)> {
    let ExprKind::Binary(binop, lhs, rhs) = cond.kind else {
        return None;
    };

    match binop.node {
        BinOpKind::Ne | BinOpKind::Lt if is_zero(lhs) => Some((rhs, NonZeroBranch::Then)),
        BinOpKind::Ne | BinOpKind::Gt if is_zero(rhs) => Some((lhs, NonZeroBranch::Then)),
        BinOpKind::Eq if is_zero(lhs) => Some((rhs, NonZeroBranch::Else)),
        BinOpKind::Eq if is_zero(rhs) => Some((lhs, NonZeroBranch::Else)),
        _ => None,
    }
}

fn branch_block<'tcx>(
    then: &'tcx Expr<'tcx>,
    r#else: Option<&'tcx Expr<'tcx>>,
    branch: NonZeroBranch,
) -> Option<&'tcx Block<'tcx>> {
    match branch {
        NonZeroBranch::Then => {
            if let ExprKind::Block(block, _) = then.kind {
                Some(block)
            } else {
                None
            }
        },
        NonZeroBranch::Else => match r#else.map(|expr| &expr.kind) {
            Some(ExprKind::Block(block, _)) => Some(block),
            _ => None,
        },
    }
}

fn is_zero(expr: &Expr<'_>) -> bool {
    is_integer_literal(expr, 0)
}

fn is_unsigned(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(cx.typeck_results().expr_ty(expr).peel_refs().kind(), ty::Uint(_))
}

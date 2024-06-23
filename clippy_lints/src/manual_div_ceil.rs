use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::SpanlessEq;
use rustc_ast::{BinOpKind, LitKind};
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for an expression like `(x + (y - 1)) / y` which is a common manual reimplementation
    /// of `x.div_ceil(y)`.
    ///
    /// ### Why is this bad?
    /// It's simpler and more readable.
    ///
    /// ### Example
    /// ```no_run
    /// let x: i32 = 7;
    /// let y: i32 = 4;
    /// let div = (x + (y - 1)) / y;
    /// ```
    /// Use instead:
    /// ```no_run
    /// #![feature(int_roundings)]
    /// let x: i32 = 7;
    /// let y: i32 = 4;
    /// let div = x.div_ceil(y);
    /// ```
    #[clippy::version = "1.81.0"]
    pub MANUAL_DIV_CEIL,
    complexity,
    "manually reimplementing `div_ceil`"
}

declare_lint_pass!(ManualDivCeil => [MANUAL_DIV_CEIL]);

impl LateLintPass<'_> for ManualDivCeil {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let mut applicability = Applicability::MachineApplicable;

        if let ExprKind::Binary(div_op, div_lhs, div_rhs) = expr.kind
            && check_int_ty(cx, div_lhs)
            && check_int_ty(cx, div_rhs)
            && div_op.node == BinOpKind::Div
        {
            // (x + (y - 1)) / y
            if let ExprKind::Binary(add_op, add_lhs, add_rhs) = div_lhs.kind
                && add_op.node == BinOpKind::Add
                && let ExprKind::Binary(sub_op, sub_lhs, sub_rhs) = add_rhs.kind
                && sub_op.node == BinOpKind::Sub
                && check_literal(sub_rhs)
                && check_eq_path_segment(cx, sub_lhs, div_rhs)
            {
                build_suggestion(cx, expr, add_lhs, div_rhs, &mut applicability);
                return;
            }

            // ((y - 1) + x) / y
            if let ExprKind::Binary(add_op, add_lhs, add_rhs) = div_lhs.kind
                && add_op.node == BinOpKind::Add
                && let ExprKind::Binary(sub_op, sub_lhs, sub_rhs) = add_lhs.kind
                && sub_op.node == BinOpKind::Sub
                && check_literal(sub_rhs)
                && check_eq_path_segment(cx, sub_lhs, div_rhs)
            {
                build_suggestion(cx, expr, add_rhs, div_rhs, &mut applicability);
                return;
            }

            // (x + y - 1) / y
            if let ExprKind::Binary(sub_op, sub_lhs, sub_rhs) = div_lhs.kind
                && sub_op.node == BinOpKind::Sub
                && let ExprKind::Binary(add_op, add_lhs, add_rhs) = sub_lhs.kind
                && add_op.node == BinOpKind::Add
                && check_literal(sub_rhs)
                && check_eq_path_segment(cx, add_rhs, div_rhs)
            {
                build_suggestion(cx, expr, add_lhs, div_rhs, &mut applicability);
            }
        }
    }
}

fn check_int_ty(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let expr_ty = cx.typeck_results().expr_ty(expr);
    matches!(expr_ty.peel_refs().kind(), ty::Int(_) | ty::Uint(_))
}

fn check_literal(expr: &Expr<'_>) -> bool {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Int(Pu128(1), _) = lit.node
    {
        return true;
    }
    false
}

fn check_eq_path_segment(cx: &LateContext<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) -> bool {
    if let ExprKind::Path(QPath::Resolved(_, lhs_path)) = lhs.kind
        && let ExprKind::Path(QPath::Resolved(_, rhs_path)) = rhs.kind
        && SpanlessEq::new(cx).eq_path_segment(&lhs_path.segments[0], &rhs_path.segments[0])
    {
        return true;
    }
    false
}

fn build_suggestion(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    lhs: &Expr<'_>,
    rhs: &Expr<'_>,
    applicability: &mut Applicability,
) {
    let dividend_snippet = snippet_with_applicability(cx, lhs.span.source_callsite(), "..", applicability);
    let divisor_snippet = snippet_with_applicability(cx, rhs.span.source_callsite(), "..", applicability);

    let sugg = format!("{dividend_snippet}.div_ceil({divisor_snippet})");

    span_lint_and_sugg(
        cx,
        MANUAL_DIV_CEIL,
        expr.span,
        "manually reimplementing `div_ceil`",
        "consider using `.div_ceil()`",
        sugg,
        *applicability,
    );
}

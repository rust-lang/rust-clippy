use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sugg::Sugg;
use clippy_utils::{is_mutable, path_to_local};
use rustc_ast::{BinOpKind, LitIntType, LitKind};
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self};
use rustc_session::impl_lint_pass;
use std::fmt::Write;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual re-implementations of checked subtraction.
    ///
    /// ### Why is this bad?
    /// Manually re-implementing checked subtraction can be error-prone and less readable.
    /// Using the standard library method `.checked_sub()` is clearer and less likely to contain bugs.
    ///
    /// ### Example
    /// ```rust
    /// // Bad: Manual implementation of checked subtraction
    /// fn get_remaining_items(total: u32, used: u32) -> Option<u32> {
    ///     if total >= used {
    ///         Some(total - used)
    ///     } else {
    ///         None
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// // Good: Using the standard library's checked_sub
    /// fn get_remaining_items(total: u32, used: u32) -> Option<u32> {
    ///     total.checked_sub(used)
    /// }
    /// ```
    #[clippy::version = "1.86.0"]
    pub MANUAL_CHECKED_SUB,
    complexity,
    "Checks for manual re-implementations of checked subtraction."
}

pub struct ManualCheckedSub {
    msrv: Msrv,
}

impl ManualCheckedSub {
    #[must_use]
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl_lint_pass!(ManualCheckedSub => [MANUAL_CHECKED_SUB]);

impl<'tcx> LateLintPass<'tcx> for ManualCheckedSub {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if !self.msrv.meets(cx, msrvs::CHECKED_SUB) {
            return;
        }

        if let Some(if_expr) = clippy_utils::higher::If::hir(expr)
            && let ExprKind::Binary(op, lhs, rhs) = if_expr.cond.kind
            && !(matches!(lhs.kind, ExprKind::Lit(_)) && matches!(rhs.kind, ExprKind::Lit(_)))
            && is_unsigned_int(cx, lhs)
            && is_unsigned_int(cx, rhs)
        {
            // Skip if either non-literal operand is mutable
            if (!matches!(lhs.kind, ExprKind::Lit(_)) && is_mutable(cx, lhs))
                || (!matches!(rhs.kind, ExprKind::Lit(_)) && is_mutable(cx, rhs))
            {
                return;
            }

            if let BinOpKind::Ge | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Lt = op.node {
                SubExprVisitor {
                    cx,
                    if_expr: expr,
                    if_body_block: if_expr.then,
                    else_block: if_expr.r#else,
                    condition_lhs: lhs,
                    condition_rhs: rhs,
                    condition_op: op.node,
                }
                .visit_expr(if_expr.then);
            }
        }
    }
}

struct SubExprVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    if_expr: &'tcx Expr<'tcx>,
    if_body_block: &'tcx Expr<'tcx>,
    else_block: Option<&'tcx Expr<'tcx>>,
    condition_lhs: &'tcx Expr<'tcx>,
    condition_rhs: &'tcx Expr<'tcx>,
    condition_op: BinOpKind,
}

impl<'tcx> Visitor<'tcx> for SubExprVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        if let ExprKind::Binary(op, sub_lhs, sub_rhs) = expr.kind
            && let BinOpKind::Sub = op.node
        {
            if let ExprKind::Lit(lit) = self.condition_lhs.kind
                && self.condition_op == BinOpKind::Lt
                && let LitKind::Int(Pu128(0), _) = lit.node
                && (is_referencing_same_variable(sub_lhs, self.condition_rhs))
            {
                self.emit_lint(expr, Some(sub_rhs));
            }

            if let ExprKind::Lit(lit) = self.condition_rhs.kind
                && self.condition_op == BinOpKind::Gt
                && let LitKind::Int(Pu128(0), _) = lit.node
                && (is_referencing_same_variable(sub_lhs, self.condition_lhs))
            {
                self.emit_lint(expr, Some(sub_rhs));
            }

            if self.condition_op == BinOpKind::Ge
                && (is_referencing_same_variable(sub_lhs, self.condition_lhs)
                    || are_literals_equal(sub_lhs, self.condition_lhs))
                && (is_referencing_same_variable(sub_rhs, self.condition_rhs)
                    || are_literals_equal(sub_rhs, self.condition_rhs))
            {
                self.emit_lint(expr, None);
            }

            if self.condition_op == BinOpKind::Le
                && (is_referencing_same_variable(sub_lhs, self.condition_rhs)
                    || are_literals_equal(sub_lhs, self.condition_rhs))
                && (is_referencing_same_variable(sub_rhs, self.condition_lhs)
                    || are_literals_equal(sub_rhs, self.condition_lhs))
            {
                self.emit_lint(expr, None);
            }
        }

        walk_expr(self, expr);
    }
}

impl<'tcx> SubExprVisitor<'_, 'tcx> {
    fn emit_lint(&mut self, expr: &Expr<'tcx>, sub_rhs_expr: Option<&'tcx Expr<'tcx>>) {
        let mut applicability = Applicability::MachineApplicable;

        let body_snippet = snippet_with_applicability(self.cx, self.if_body_block.span, "..", &mut applicability);

        let binary_expr_snippet = snippet_with_applicability(self.cx, expr.span, "..", &mut applicability);
        let updated_usage_context_snippet = body_snippet.as_ref().replace(binary_expr_snippet.as_ref(), "result");

        let lhs_snippet = Sugg::hir(self.cx, self.condition_lhs, "..").maybe_paren();

        let rhs_snippet = match sub_rhs_expr {
            Some(sub_rhs) => Sugg::hir(self.cx, sub_rhs, "..").maybe_paren(),
            None => Sugg::hir(self.cx, self.condition_rhs, "..").maybe_paren(),
        };

        // TODO: Check result variable is not already in scope
        let mut suggestion = match self.condition_op {
            BinOpKind::Le => {
                format!(
                    "if let Some(result) = {rhs_snippet}.checked_sub({lhs_snippet}) {updated_usage_context_snippet}"
                )
            },

            BinOpKind::Lt => {
                format!(
                    "if let Some(result) = {}.checked_sub({rhs_snippet}) {updated_usage_context_snippet}",
                    Sugg::hir(self.cx, self.condition_rhs, "..").maybe_paren()
                )
            },
            _ => {
                format!(
                    "if let Some(result) = {lhs_snippet}.checked_sub({rhs_snippet}) {updated_usage_context_snippet}"
                )
            },
        };

        if let Some(else_expr) = self.else_block {
            let else_snippet = snippet_with_applicability(self.cx, else_expr.span, "..", &mut applicability);
            write!(suggestion, " else {else_snippet}").unwrap();
        }

        span_lint_and_sugg(
            self.cx,
            MANUAL_CHECKED_SUB,
            self.if_expr.span,
            "manual re-implementation of checked subtraction",
            "consider using `.checked_sub()`",
            suggestion,
            applicability,
        );
    }
}

fn is_unsigned_int<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    let expr_type = cx.typeck_results().expr_ty(expr).peel_refs();
    if matches!(expr_type.kind(), ty::Uint(_)) {
        return true;
    }

    false
}

fn are_literals_equal<'tcx>(expr1: &Expr<'tcx>, expr2: &Expr<'tcx>) -> bool {
    if let (ExprKind::Lit(lit1), ExprKind::Lit(lit2)) = (&expr1.kind, &expr2.kind)
        && let (LitKind::Int(val1, suffix1), LitKind::Int(val2, suffix2)) = (&lit1.node, &lit2.node)
    {
        return val1 == val2
            && suffix1 == suffix2
            && *suffix1 != LitIntType::Unsuffixed
            && *suffix2 != LitIntType::Unsuffixed;
    }

    false
}

fn is_referencing_same_variable<'tcx>(expr1: &'tcx Expr<'_>, expr2: &'tcx Expr<'_>) -> bool {
    if let (Some(id1), Some(id2)) = (path_to_local(expr1), path_to_local(expr2)) {
        return id1 == id2;
    }

    false
}

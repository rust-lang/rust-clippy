use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::{BinOpKind, LitIntType, LitKind};
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual re-implementations of checked subtraction.
    ///
    /// ### Why is this bad?
    /// Manually re-implementing checked subtraction can be error-prone and less readable.
    /// Using the standard library method `.checked_sub()` is clearer and less likely to contain bugs.
    ///
    /// ### Example
    /// ```no_run
    /// if a >= b {
    ///     a - b
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// a.checked_sub(b)
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
        Self {
            msrv: conf.msrv.clone(),
        }
    }
}

impl_lint_pass!(ManualCheckedSub => [MANUAL_CHECKED_SUB]);

impl<'tcx> LateLintPass<'tcx> for ManualCheckedSub {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if !self.msrv.meets(cx, msrvs::MANUAL_CHECKED_SUB) {
            return;
        }

        let applicability = Applicability::MachineApplicable;

        if let ExprKind::If(drop_temp_expr, body_block, else_block) = expr.kind
            && let ExprKind::DropTemps(condition_expr) = drop_temp_expr.kind
            && let ExprKind::Binary(op, lhs, rhs) = condition_expr.kind
            && is_unsigned_int(cx, &lhs)
            && is_unsigned_int(cx, &rhs)
        {
            if let BinOpKind::Ge = op.node {
                SubExprVisitor {
                    cx,
                    applicability,
                    if_expr: expr,
                    if_body_block: body_block,
                    else_block,
                    condition_lhs: &lhs,
                    condition_rhs: &rhs,
                }
                .visit_expr(body_block)
            }

            if let BinOpKind::Gt = op.node {
                if let ExprKind::Lit(lit) = &rhs.kind
                    && let LitKind::Int(Pu128(0), _) = lit.node
                {
                    SubExprVisitor {
                        cx,
                        applicability,
                        if_expr: expr,
                        if_body_block: body_block,
                        else_block,
                        condition_lhs: &lhs,
                        condition_rhs: &rhs,
                    }
                    .visit_expr(body_block)
                }
            }
        }
    }
}

struct SubExprVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    if_expr: &'tcx Expr<'tcx>,
    if_body_block: &'tcx Expr<'tcx>,
    else_block: Option<&'tcx Expr<'tcx>>,
    applicability: Applicability,
    condition_lhs: &'tcx Expr<'tcx>,
    condition_rhs: &'tcx Expr<'tcx>,
}

impl<'tcx> Visitor<'tcx> for SubExprVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        if let ExprKind::Binary(op, sub_lhs, sub_rhs) = expr.kind
            && let BinOpKind::Sub = op.node
        {
            if let ExprKind::Lit(lit) = self.condition_rhs.kind
                && let LitKind::Int(Pu128(0), _) = lit.node
                && (is_referencing_same_variable(&sub_lhs, self.condition_lhs)
                    || are_literals_equal(&sub_lhs, self.condition_lhs))
            {
                self.build_suggession(expr);
            } else {
                if (is_referencing_same_variable(&sub_lhs, self.condition_lhs)
                    || are_literals_equal(&sub_lhs, self.condition_lhs))
                    && (is_referencing_same_variable(&sub_rhs, self.condition_rhs)
                        || are_literals_equal(&sub_rhs, self.condition_rhs))
                {
                    self.build_suggession(expr);
                }
            }
        }

        walk_expr(self, expr);
    }
}

impl<'a, 'tcx> SubExprVisitor<'a, 'tcx> {
    fn build_suggession(&mut self, expr: &Expr<'tcx>) {
        // if let Some(else_expr) = self.else_block {
        //     println!("ELSE BLOCK in suggestion:::: {:#?}", else_expr);
        // }

        let body_snippet = snippet_with_applicability(self.cx, self.if_body_block.span, "..", &mut self.applicability);

        let binary_expr_snippet = snippet_with_applicability(self.cx, expr.span, "..", &mut self.applicability);
        let updated_usage_context_snippet = body_snippet.as_ref().replace(binary_expr_snippet.as_ref(), "result");
        // let else_snippet = snippet_with_applicability(self.cx, self.if_else_block.span, "..", &mut
        // self.applicability);

        let lhs_snippet = snippet_with_applicability(self.cx, self.condition_lhs.span, "..", &mut self.applicability);
        let rhs_snippet = snippet_with_applicability(self.cx, self.condition_rhs.span, "..", &mut self.applicability);

        let lhs_needs_parens = matches!(self.condition_lhs.kind, ExprKind::Cast(..));

        let mut suggestion = if lhs_needs_parens {
            format!(
                "if let Some(result) = ({}).checked_sub({}) {}",
                lhs_snippet, rhs_snippet, updated_usage_context_snippet
            )
        } else {
            format!(
                "if let Some(result) = {}.checked_sub({})  {}",
                lhs_snippet, rhs_snippet, updated_usage_context_snippet
            )
        };

        if let Some(else_expr) = self.else_block {
            let else_snippet = snippet_with_applicability(self.cx, else_expr.span, "..", &mut self.applicability);
            suggestion.push_str(&format!(" else {}", else_snippet));
        }

        span_lint_and_sugg(
            self.cx,
            MANUAL_CHECKED_SUB,
            self.if_expr.span,
            "manual re-implementation of checked subtraction",
            "consider using `.checked_sub()`",
            suggestion,
            self.applicability,
        );
    }
}

fn is_unsigned_int<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>) -> bool {
    let expr_type = cx.typeck_results().expr_ty(expr).peel_refs();
    if matches!(expr_type.kind(), ty::Uint(_)) {
        return true;
    }

    if let ExprKind::Lit(lit) = &expr.kind {
        if let LitKind::Int(_, suffix) = &lit.node {
            if let LitIntType::Unsuffixed = suffix {
                return true;
            }
        }
    }
    false
}

fn are_literals_equal<'tcx>(expr1: &Expr<'tcx>, expr2: &Expr<'tcx>) -> bool {
    if let (ExprKind::Lit(lit1), ExprKind::Lit(lit2)) = (&expr1.kind, &expr2.kind) {
        if let (LitKind::Int(val1, suffix1), LitKind::Int(val2, suffix2)) = (&lit1.node, &lit2.node) {
            return val1 == val2
                && suffix1 == suffix2
                && *suffix1 != LitIntType::Unsuffixed
                && *suffix2 != LitIntType::Unsuffixed;
        }
    }
    false
}
fn is_referencing_same_variable<'tcx>(expr1: &'tcx Expr<'_>, expr2: &'tcx Expr<'_>) -> bool {
    if let ExprKind::Cast(cast_expr1, _) = &expr1.kind {
        if let Some(cast_expr1_path) = get_resolved_path_id(cast_expr1) {
            if let ExprKind::Cast(cast_expr2, _) = &expr2.kind {
                if let Some(cast_expr2_path) = get_resolved_path_id(cast_expr2) {
                    return cast_expr1_path == cast_expr2_path;
                }
            } else {
                if let Some(expr2_path) = get_resolved_path_id(expr2) {
                    return cast_expr1_path == expr2_path;
                };
                return false;
            }
        }
    } else if let ExprKind::Cast(cast_expr2, _) = &expr2.kind {
        if let Some(cast_expr2_path) = get_resolved_path_id(cast_expr2) {
            if let Some(expr1_path) = get_resolved_path_id(expr1) {
                return expr1_path == cast_expr2_path;
            };
            return false;
        }
    }

    let expr1_path = get_resolved_path_id(expr1);
    let expr2_path = get_resolved_path_id(expr2);

    if let (Some(expr1_path), Some(expr2_path)) = (expr1_path, expr2_path) {
        expr1_path == expr2_path
    } else {
        false
    }
}

fn get_resolved_path_id<'tcx>(expr: &'tcx Expr<'_>) -> Option<Res> {
    if let ExprKind::Path(QPath::Resolved(_, path)) = &expr.kind {
        path.segments.first().and_then(|segment| Some(segment.res))
    } else {
        None
    }
}

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::visitors::for_each_expr_without_closures;
use clippy_utils::{get_parent_expr, higher, peel_blocks};
use core::ops::ControlFlow;
use rustc_ast::ast::LitKind;
use rustc_ast::BinOpKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects `if`-then-`else` that can be replaced with `&&`.
    ///
    /// ### Why is this bad?
    /// `&&` is simpler than `if`-then-`else`.
    ///
    /// ### Example
    /// ```ignore
    /// if a {
    ///     b
    /// } else {
    ///     false
    /// }
    /// ```
    /// Use instead:
    /// ```ignore
    /// a && b
    /// ```
    #[clippy::version = "1.80.0"]
    pub MANUAL_AND,
    complexity,
    "this `if`-then-`else` that can be replaced with `&&`."
}

declare_clippy_lint! {
    /// ### What it does
    /// Detects `if`-then-`else` that can be replaced with `||`.
    ///
    /// ### Why is this bad?
    /// `||` is simpler than `if`-then-`else`.
    ///
    /// ### Example
    /// ```ignore
    /// if a {
    ///     true
    /// } else {
    ///     b
    /// }
    /// ```
    /// Use instead:
    /// ```ignore
    /// a || b
    /// ```
    #[clippy::version = "1.80.0"]
    pub MANUAL_OR,
    complexity,
    "this `if`-then-`else` expression can be simplified with `||`"
}

declare_lint_pass!(ManualAndOr => [MANUAL_AND, MANUAL_OR]);

fn extract_final_expression_snippet<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<String> {
    if let ExprKind::Block(block, _) = expr.kind {
        if let Some(final_expr) = block.expr {
            return cx.sess().source_map().span_to_snippet(final_expr.span).ok();
        }
    }
    cx.sess().source_map().span_to_snippet(expr.span).ok()
}

fn fetch_bool_expr(expr: &Expr<'_>) -> Option<bool> {
    if let ExprKind::Lit(lit_ptr) = peel_blocks(expr).kind {
        if let LitKind::Bool(value) = lit_ptr.node {
            return Some(value);
        }
    }
    None
}

fn contains_or(cond: &Expr<'_>) -> bool {
    for_each_expr_without_closures(cond, |e| {
        if let ExprKind::Binary(ref n, _, _) = e.kind {
            if n.node == BinOpKind::Or {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}

fn check_and<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, cond: &Expr<'tcx>, then: &Expr<'tcx>) {
    if let Some(parent) = get_parent_expr(cx, expr) {
        if let ExprKind::If(_, _, _) = parent.kind {
            return;
        }
    }
    if contains_or(cond) || contains_or(then) || fetch_bool_expr(then).is_some() {
        return;
    }
    if match then.kind {
        ExprKind::Block(block, _) => !block.stmts.is_empty(),
        _ => false,
    } {
        return;
    }

    let applicability = Applicability::MachineApplicable;
    let cond_snippet = cx
        .sess()
        .source_map()
        .span_to_snippet(cond.span)
        .unwrap_or_else(|_| "..".to_string());

    let then_snippet = extract_final_expression_snippet(cx, then).unwrap_or_else(|| "..".to_string());
    let suggestion = format!("{cond_snippet} && {then_snippet}");
    span_lint_and_sugg(
        cx,
        MANUAL_AND,
        expr.span,
        "this `if`-then-`else` expression can be simplified with `&&`",
        "try",
        suggestion,
        applicability,
    );
}

fn check_or<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, cond: &Expr<'tcx>, else_expr: &Expr<'tcx>) {
    if matches!(else_expr.kind, ExprKind::If(..)) || fetch_bool_expr(else_expr).is_some() {
        return;
    }
    if match else_expr.kind {
        ExprKind::Block(block, _) => !block.stmts.is_empty(),
        _ => false,
    } {
        return;
    }

    let applicability = Applicability::MachineApplicable;
    let cond_snippet = cx
        .sess()
        .source_map()
        .span_to_snippet(cond.span)
        .unwrap_or_else(|_| "..".to_string());

    let else_snippet = extract_final_expression_snippet(cx, else_expr).unwrap_or_else(|| "..".to_string());
    let suggestion = format!("{cond_snippet} || {else_snippet}");
    span_lint_and_sugg(
        cx,
        MANUAL_OR,
        expr.span,
        "this `if`-then-`else` expression can be simplified with `||`",
        "try",
        suggestion,
        applicability,
    );
}

impl<'tcx> LateLintPass<'tcx> for ManualAndOr {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if let Some(higher::If {
            cond,
            then,
            r#else: Some(else_expr),
        }) = higher::If::hir(expr)
        {
            if let Some(false) = fetch_bool_expr(else_expr) {
                check_and(cx, expr, cond, then);
            } else if let Some(true) = fetch_bool_expr(then) {
                check_or(cx, expr, cond, else_expr);
            }
        }
    }
}

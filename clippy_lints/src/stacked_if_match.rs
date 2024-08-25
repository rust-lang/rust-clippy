use clippy_utils::source::snippet;
use rustc_middle::lint::in_external_macro;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_errors::Applicability;
use clippy_utils::visitors::{for_each_expr, Descend};
use clippy_utils::diagnostics::span_lint_and_sugg;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if if` and `match match`.
    ///
    /// ### Why is this bad?
    /// `if if` and `match match` are hard to read.
    ///
    /// ### Example
    /// ```no_run
    /// if if a == b {
    ///     c == d
    /// } else {
    ///     e == f
    /// } {
    ///     println!("true");
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let result = if a == b {
    ///     c == d
    /// } else {
    ///     e == f
    /// };
    ///
    /// if result {
    ///     println!("true");
    /// }
    /// ```
    #[clippy::version = "1.82.0"]
    pub STACKED_IF_MATCH,
    style,
    "`if if` and `match match` that can be eliminated"
}

declare_lint_pass!(StackedIfMatch => [STACKED_IF_MATCH]);

impl<'tcx> LateLintPass<'tcx> for StackedIfMatch {
   fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() || in_external_macro(cx.sess(), expr.span) {
            return;
        }

        let Some((cond, keyword)) = (match expr.kind {
            ExprKind::If(if_expr, _, _) => Some((if_expr, "if")),
            ExprKind::Match(match_expr, _, MatchSource::Normal) => Some((match_expr, "match")),
            _ => None,
        }) else {
            return;
        };

        let cond_snippet = snippet(cx, cond.span, "");
        if !cond_snippet.starts_with("if") && !cond_snippet.starts_with("match") {
            return;
        }

        for_each_expr(cx, cond, |sub_expr| {
            if matches!(sub_expr.kind, ExprKind::DropTemps(..)) {
                return ControlFlow::Continue(Descend::Yes);
            }

            if !sub_expr.span.eq_ctxt(expr.span) || sub_expr.span.lo() != cond.span.lo() {
                return ControlFlow::Continue(Descend::No);
            }

            if (keyword == "if" && matches!(sub_expr.kind, ExprKind::If(..)))
                || (keyword == "match" && matches!(sub_expr.kind, ExprKind::Match(.., MatchSource::Normal))) {
                let inner_snippet = snippet(cx, sub_expr.span, "..");
                span_lint_and_sugg(
                    cx,
                    STACKED_IF_MATCH,
                    expr.span.with_hi(sub_expr.span.hi()),
                    format!("avoid using `{keyword} {keyword}`"),
                    format!("try binding inner `{keyword}` with `let`"),
                    format!("let result = {inner_snippet}; {keyword} result"),
                    Applicability::MachineApplicable,
                );
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(Descend::Yes)
            }
        });
    }
}

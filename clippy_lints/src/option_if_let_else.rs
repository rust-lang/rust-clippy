use crate::utils::sugg::Sugg;
use crate::utils::{match_qpath, match_type, paths, span_lint, span_lint_and_sugg};
use if_chain::if_chain;

use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:**
    /// Detects people who use `if let Some(v) = ... { y } else { x }`
    /// when they could use `Option::map_or` instead.
    ///
    /// **Why is this bad?**
    /// Using the dedicated function in the Option class is clearer and
    /// more concise than an if let
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// let var: Option<u32> = Some(5u32);
    /// let _ = if let Some(foo) = var {
    ///     foo
    /// } else {
    ///     5
    /// };
    /// ```
    ///
    /// should be
    ///
    /// ```rust
    /// let var: Option<u32> = Some(5u32);
    /// let _ = var.map_or(5, |foo| foo);
    /// ```
    pub OPTION_IF_LET_ELSE,
    style,
    "reimplementation of Option::map_or"
}

declare_lint_pass!(OptionIfLetElse => [OPTION_IF_LET_ELSE]);

/// If this is the option if let/else thing we're detecting, then this
/// function returns Some(Option<_> compared, expression if Option is
/// Some, expression if Option is None). Otherwise, it returns None
fn detect_option_if_let_else<'a>(
    cx: &LateContext<'_, '_>,
    expr: &'a Expr<'_>,
) -> Option<(&'a Expr<'a>, &'a Expr<'a>, &'a Expr<'a>)> {
    if let ExprKind::Match(let_body, arms, MatchSource::IfLetDesugar{contains_else_clause: true}) = &expr.kind {
        dbg!("Found if let/else stmt");
        if arms.len() == 2 {
            dbg!("If let/else statement has two arms");
            if match_type(cx, &cx.tables.expr_ty(let_body), &paths::OPTION) {
                dbg!("rhs of if let is Option<T>");
                if let PatKind::TupleStruct(path, &[inner_pat], _) = &arms[0].pat.kind {
                    dbg!("arm0 is TupleStruct");
                    if let PatKind::Wild | PatKind::Binding(..) = &inner_pat.kind {
                        dbg!("inside of Some(x) matches anything");
                        let (some_body, none_body) = if match_qpath(path, &paths::OPTION_SOME) {
                            (arms[0].body, arms[1].body)
                        } else {
                            (arms[1].body, arms[0].body)
                        };
                        return Some((let_body, some_body, none_body));
                    }
                }
            }
        }
    }
    None
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for OptionIfLetElse {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr<'_>) {
        span_lint(
            cx,
            OPTION_IF_LET_ELSE,
            expr.span,
            "OptionIfLetElse::check_expr was run on this span",
        );
        return;
        if let Some((option, map, else_func)) = detect_option_if_let_else(cx, expr) {
            span_lint_and_sugg(
                cx,
                OPTION_IF_LET_ELSE,
                expr.span,
                "use Option::map_or_else instead of an if let/else",
                "try",
                format!(
                    "{}.map_or_else({}, {})",
                    Sugg::hir(cx, option, ".."),
                    Sugg::hir(cx, else_func, ".."),
                    Sugg::hir(cx, map, "..")
                ),
                Applicability::MachineApplicable,
            );
        }
    }
}

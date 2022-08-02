use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::eager_or_lazy::switch_to_eager_eval;
use clippy_utils::higher::If;
use clippy_utils::is_lang_ctor;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if` expressions where one branch immediately returns None, and the other branch
    /// returns `Some(_)` (with or without side-effects).
    ///
    /// ### Why is this bad?
    /// Contributes to code noise, and can be simpler expressed using methods on `bool`.
    ///
    /// ### Example
    /// ```rust
    /// let _ = if x {
    ///     Some("asdf")
    /// } else {
    ///     None
    /// };
    /// ```
    /// Use instead:
    /// ```rust
    /// let _ = x.then_some("asdf");
    /// ```
    #[clippy::version = "1.64.0"]
    pub IF_NONE_BLOCKS,
    pedantic,
    "if expression containing a branch that just returns `None`"
}
declare_lint_pass!(IfNoneBlocks => [IF_NONE_BLOCKS]);

impl<'tcx> LateLintPass<'tcx> for IfNoneBlocks {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(if_expr) = If::hir(expr)
            && let Some(else_block) = if_expr.r#else
            && let Some(negation) = check_lint(cx, if_expr.then, else_block)
        {
            let negation_msg = negation.then_some("first negating the condition, then ").unwrap_or_default();
            let method_name = switch_to_eager_eval(cx, expr).then_some("then_some").unwrap_or("then");
            span_lint_and_help(
                cx,
                IF_NONE_BLOCKS,
                expr.span,
                "if expression simply returns `None` in one of its branches",
                None,
                &format!("consider {}using `bool::{}`", negation_msg, method_name),
            );
        }
    }
}

// Checks if the lint applies and whether or not to recommend negating the condition.
fn check_lint<'tcx>(cx: &LateContext<'tcx>, then: &'tcx Expr<'tcx>, r#else: &'tcx Expr<'tcx>) -> Option<bool> {
    if check_none(cx, r#else) && check_some(cx, then) {
        Some(false) // None in `else` block
    } else if check_none(cx, then) && check_some(cx, r#else) {
        Some(true) // None in `then` block
    } else {
        None // lint does not apply
    }
}

// Checks if an expression returns `Some(_)`; side-effects and extra statements are allowed.
fn check_some<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::Block(block, _) = expr.kind
        && let Some(expr) = block.expr
        && let ExprKind::Call(call, _) = &expr.peel_blocks().kind
        && let ExprKind::Path(q) = &call.kind
    {
        return is_lang_ctor(cx, q, LangItem::OptionSome)
    }
    false
}

// Checks if an expression immediately returns `None`.
fn check_none<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::Path(q) = &expr.peel_blocks().kind {
        return is_lang_ctor(cx, q, LangItem::OptionNone);
    }
    false
}

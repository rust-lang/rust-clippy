use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{FileRangeExt, SpanExt};
use rustc_ast::ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty `else` branches.
    ///
    /// ### Why is this bad?
    /// An empty else branch does nothing and can be removed.
    ///
    /// ### Example
    /// ```no_run
    ///# fn check() -> bool { true }
    /// if check() {
    ///     println!("Check successful!");
    /// } else {
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    ///# fn check() -> bool { true }
    /// if check() {
    ///     println!("Check successful!");
    /// }
    /// ```
    #[clippy::version = "1.72.0"]
    pub NEEDLESS_ELSE,
    style,
    "empty else branch"
}
declare_lint_pass!(NeedlessElse => [NEEDLESS_ELSE]);

impl EarlyLintPass for NeedlessElse {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let ExprKind::If(_, then_block, Some(else_clause)) = &expr.kind
            && let ExprKind::Block(block, _) = &else_clause.kind
            && !then_block.span.from_expansion()
            && !expr.span.from_expansion()
            && !else_clause.span.from_expansion()
            && block.stmts.is_empty()
            // Only take the span of `else { .. }` if no comments/cfgs/macros exist.
            && let Some(lint_sp) = else_clause.span.map_range(cx, |scx, range| {
                range.extend_start_to(scx, then_block.span.hi_ctxt())?
                    .map_range_text(scx, |src| {
                        let src = src.trim_start();
                        (src.strip_prefix("else")?
                            .trim_start()
                            .strip_prefix('{')?
                            .trim_start() == "}").then_some(src)
                    })
            })
        {
            span_lint_and_sugg(
                cx,
                NEEDLESS_ELSE,
                lint_sp,
                "this `else` branch is empty",
                "you can remove it",
                String::new(),
                Applicability::MachineApplicable,
            );
        }
    }
}

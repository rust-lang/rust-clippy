use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_span_if, is_span_match};
use rustc_ast::ast::{Expr, ExprKind, MatchKind};
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if if`: an `if` whose condition starts with another `if`.
    ///
    /// Only the start of the condition is checked, so `if if a { b } else { c } == d` is
    /// linted but `if d == if a { b } else { c }` is not. Parentheses around the inner `if`
    /// or a `let` binding silence the lint. `if match` is fine, the different keywords keep
    /// it readable.
    ///
    /// ### Why is this bad?
    /// Two `if`s in a row make it hard to see where the outer condition starts and ends.
    ///
    /// ### Example
    /// ```no_run
    /// # let value = 1;
    /// if if value > 0 { value < 10 } else { false } {
    ///     println!("single digit");
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let value = 1;
    /// let is_single_digit = if value > 0 { value < 10 } else { false };
    /// if is_single_digit {
    ///     println!("single digit");
    /// }
    /// ```
    #[clippy::version = "1.100.0"]
    pub STACKED_IF,
    style,
    "`if` expression stacked inside the condition of another `if`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `match match`: a `match` whose scrutinee starts with another `match`.
    ///
    /// Only the start of the scrutinee is checked, so `match match a { .. } + 1` is linted
    /// but `match 1 + match a { .. }` is not. Parentheses around the inner `match` or a `let`
    /// binding silence the lint. `match if` is fine, the different keywords keep it readable.
    ///
    /// ### Why is this bad?
    /// Two `match`es in a row make it hard to see where the outer scrutinee starts and ends.
    ///
    /// ### Example
    /// ```no_run
    /// # let value = 1;
    /// match match value {
    ///     0 => 1,
    ///     _ => 2,
    /// } {
    ///     1 => println!("one"),
    ///     _ => println!("other"),
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let value = 1;
    /// let result = match value {
    ///     0 => 1,
    ///     _ => 2,
    /// };
    /// match result {
    ///     1 => println!("one"),
    ///     _ => println!("other"),
    /// }
    /// ```
    ///
    /// If the inner `match` borrows from a temporary, bind that temporary to a local first,
    /// or the borrow will not live long enough.
    #[clippy::version = "1.100.0"]
    pub STACKED_MATCH,
    style,
    "`match` expression stacked inside the scrutinee of another `match`"
}

declare_lint_pass!(StackedIfMatch => [STACKED_IF, STACKED_MATCH]);

impl EarlyLintPass for StackedIfMatch {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if !expr.span.from_expansion()
            && let Some((inner, lint, keyword, position)) = stacked_inner(expr)
            && !inner.span.from_expansion()
            // Proc-macros can give weird spans; check the source really is an `if`/`match`.
            && match inner.kind {
                ExprKind::If(..) => is_span_if(cx, inner.span),
                _ => is_span_match(cx, inner.span),
            }
        {
            span_lint_and_then(
                cx,
                lint,
                inner.span,
                format!("this `{keyword}` expression is stacked inside the {position} of another `{keyword}`"),
                |diag| {
                    diag.help(format!(
                        "consider binding the inner `{keyword}` expression to a local before the outer `{keyword}`"
                    ));
                    if let ExprKind::Match(..) = inner.kind {
                        diag.help(
                            "if the inner `match` borrows from its scrutinee, bind the scrutinee to a local first",
                        );
                    }
                },
            );
        }
    }
}

/// If the condition/scrutinee of `expr` starts with the same kind of expression, returns that
/// inner expression plus the lint, keyword and position name to report.
fn stacked_inner(expr: &Expr) -> Option<(&Expr, &'static Lint, &'static str, &'static str)> {
    match &expr.kind {
        ExprKind::If(cond, ..)
            if let inner = leftmost_expr(cond)
                && let ExprKind::If(..) = inner.kind =>
        {
            Some((inner, STACKED_IF, "if", "condition"))
        },
        ExprKind::Match(scrutinee, _, MatchKind::Prefix)
            if let inner = leftmost_expr(scrutinee)
                && let ExprKind::Match(_, _, MatchKind::Prefix) = inner.kind =>
        {
            Some((inner, STACKED_MATCH, "match", "scrutinee"))
        },
        _ => None,
    }
}

/// Walks down to the sub-expression that starts at the same token as `expr`, like
/// `rustc_ast::util::classify::leading_labeled_expr`. Stops at parentheses, blocks, closures and
/// macros (including `type_ascribe!`), so those can be used to silence the lints.
fn leftmost_expr(mut expr: &Expr) -> &Expr {
    loop {
        expr = match &expr.kind {
            ExprKind::Assign(e, ..)
            | ExprKind::AssignOp(_, e, _)
            | ExprKind::Await(e, _)
            | ExprKind::Binary(_, e, _)
            | ExprKind::Call(e, _)
            | ExprKind::Cast(e, _)
            | ExprKind::Field(e, _)
            | ExprKind::Index(e, ..)
            | ExprKind::Match(e, _, MatchKind::Postfix)
            | ExprKind::Move(e, _)
            | ExprKind::Range(Some(e), ..)
            | ExprKind::Try(e)
            | ExprKind::Use(e, _) => e,
            ExprKind::MethodCall(call) => &call.receiver,
            _ => return expr,
        };
    }
}

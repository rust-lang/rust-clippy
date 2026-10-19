use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{SpanExt as _, snippet_with_applicability, snippet_with_context};
use rustc_ast::ast::{Expr, ExprKind, MethodCall};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext as _, impl_lint_pass};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary double parentheses.
    ///
    /// ### Why is this bad?
    /// This makes code harder to read and might indicate a
    /// mistake.
    ///
    /// ### Example
    /// ```no_run
    /// fn simple_double_parens() -> i32 {
    ///     ((0))
    /// }
    ///
    /// # fn foo(bar: usize) {}
    /// foo((0));
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn simple_no_parens() -> i32 {
    ///     (0)
    /// }
    ///
    /// # fn foo(bar: usize) {}
    /// foo(0);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub DOUBLE_PARENS,
    complexity,
    "Warn on unnecessary double parentheses"
}

impl_lint_pass!(DoubleParens => [DOUBLE_PARENS]);

#[derive(Default)]
pub struct DoubleParens {
    /// The redundant parentheses that have already been reported, as a pair of the outermost
    /// parentheses and the innermost one that is kept. The parentheses in between are removed by
    /// that diagnostic, so they must not be reported again.
    reported: Option<(Span, Span)>,
}

impl EarlyLintPass for DoubleParens {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        // The parentheses are already removed by an enclosing diagnostic.
        if let Some((outer, keep)) = self.reported
            && encloses_or_eq(outer, expr.span)
            && !encloses_or_eq(keep, expr.span)
        {
            return;
        }

        match &expr.kind {
            // ((..))
            // ^^^^^^ expr
            //  ^^^^  inner
            ExprKind::Paren(inner)
                if matches!(inner.kind, ExprKind::Paren(_) | ExprKind::Tup(_))
                    && expr.span.eq_ctxt(inner.span)
                    && !expr.span.in_external_macro(cx.sess().source_map())
                    && check_source(cx, inner) =>
            {
                // Every nested pair of parentheses is removed at once, so only report the outermost
                // one. `keep` is the pair that remains after the suggestion is applied.
                let keep = innermost_paren(inner);

                let mut applicability = Applicability::MachineApplicable;
                // We don't need to use `snippet_with_context` here, because:
                // - if `inner`'s `ctxt` is from macro, we don't lint in the first place (see the check above)
                // - otherwise, calling `snippet_with_applicability` on a not-from-macro span is fine
                let sugg = snippet_with_applicability(cx.sess(), keep.span, "_", &mut applicability);
                self.reported = Some((expr.span, keep.span));

                span_lint_and_sugg(
                    cx,
                    DOUBLE_PARENS,
                    expr.span,
                    "unnecessary parentheses",
                    "remove them",
                    sugg.to_string(),
                    applicability,
                );
            },

            // func((n))
            // ^^^^^^^^^ expr
            //      ^^^  arg
            //       ^   inner
            ExprKind::Call(_, args) | ExprKind::MethodCall(MethodCall { args, .. })
                if let [arg] = &**args
                    && let ExprKind::Paren(inner) = &arg.kind
                    && expr.span.eq_ctxt(arg.span)
                    && !arg.span.in_external_macro(cx.sess().source_map())
                    && check_source(cx, arg) =>
            {
                // See the comment in the branch above.
                let keep = innermost_expr(inner);

                let mut applicability = Applicability::MachineApplicable;
                let sugg = snippet_with_context(cx.sess(), keep.span, arg.span.ctxt(), "_", &mut applicability).0;
                self.reported = Some((arg.span, keep.span));

                span_lint_and_sugg(
                    cx,
                    DOUBLE_PARENS,
                    arg.span,
                    "unnecessary parentheses",
                    "remove them",
                    sugg.to_string(),
                    applicability,
                );
            },
            _ => {},
        }
    }

    fn check_expr_post(&mut self, _: &EarlyContext<'_>, expr: &Expr) {
        // The whole expression that was reported has been visited, nothing else is removed by it.
        if self.reported.is_some_and(|(outer, _)| outer == expr.span) {
            self.reported = None;
        }
    }
}

/// Returns the parentheses that remain when all the redundant parentheses around `inner` are
/// removed.
///
/// `inner` is the expression directly enclosed by the first redundant pair, so for `((((1))))` this
/// returns `(1)`, and for `(((1, 2)))` it returns `(1, 2)`, the tuple's span including the
/// parentheses that are part of it. Note that a single pair is kept, since removing it as well may
/// change how the expression is parsed.
fn innermost_paren(inner: &Expr) -> &Expr {
    let mut inner = inner;
    while let ExprKind::Paren(next) = &inner.kind
        && matches!(next.kind, ExprKind::Paren(_) | ExprKind::Tup(_))
    {
        inner = next;
    }
    inner
}

/// Returns the expression enclosed by all the redundant parentheses around `inner`.
///
/// `inner` is the expression directly enclosed by the first redundant pair. Every pair is removed
/// here, because parentheses are never needed around the sole argument of a call, so for
/// `func((((1))))` this returns `1`.
fn innermost_expr(inner: &Expr) -> &Expr {
    let mut inner = inner;
    while let ExprKind::Paren(next) = &inner.kind {
        inner = next;
    }
    inner
}

/// Returns `true` if `outer` encloses `inner` or is `inner`.
fn encloses_or_eq(outer: Span, inner: Span) -> bool {
    outer == inner || outer.contains(inner)
}

/// Check that the span does indeed look like `(  (..)  )`
fn check_source(cx: &EarlyContext<'_>, inner: &Expr) -> bool {
    if let Some(sfr) = inner.span.get_source_range(cx)
        // this is the same as `SourceFileRange::as_str`, but doesn't apply the range right away, because
        // we're interested in the source code outside it
        && let Some(src) = sfr.sf.src.as_ref().map(|src| src.as_str())
        && let Some((start, outer_after_inner)) = src.split_at_checked(sfr.range.end)
        && let Some((outer_before_inner, inner)) = start.split_at_checked(sfr.range.start)
        && outer_before_inner.trim_end().ends_with('(')
        && inner.starts_with('(')
        && inner.ends_with(')')
        && outer_after_inner.trim_start().starts_with(')')
        // Don't lint macro repetition patterns like `($($result),*)` where parens are necessary
        && !inner.trim_start_matches('(').trim_start().starts_with("$(")
    {
        true
    } else {
        false
    }
}

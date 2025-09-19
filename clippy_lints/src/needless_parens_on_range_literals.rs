use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{SpanRangeExt, snippet_with_applicability};

use rustc_ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
  /// ### What it does
  /// The lint checks for parenthesis on literals in range statements that are
  /// superfluous.
  ///
  /// ### Why is this bad?
  /// Having superfluous parenthesis makes the code less readable
  /// overhead when reading.
  ///
  /// ### Example
  ///
  /// ```no_run
  /// for i in (0)..10 {
  ///   println!("{i}");
  /// }
  /// ```
  ///
  /// Use instead:
  ///
  /// ```no_run
  /// for i in 0..10 {
  ///   println!("{i}");
  /// }
  /// ```
  #[clippy::version = "1.63.0"]
  pub NEEDLESS_PARENS_ON_RANGE_LITERALS,
  style,
  "needless parenthesis on range literals can be removed"
}

declare_lint_pass!(NeedlessParensOnRangeLiterals => [NEEDLESS_PARENS_ON_RANGE_LITERALS]);

fn check_for_parens(cx: &EarlyContext<'_>, e: &Expr, is_start: bool) {
    if let ExprKind::Paren(literal) = &e.kind
        && let ExprKind::Lit(lit) = &literal.kind
    {
        if is_start
            && lit.kind == rustc_ast::token::LitKind::Float
            && lit.suffix.is_none()
            && literal.span.check_source_text(cx, |s| s.ends_with('.'))
        {
            // don't lint `(2.)..end`, since removing the parens would result in invalid syntax
            return;
        }

        let mut applicability = Applicability::MachineApplicable;
        let suggestion = snippet_with_applicability(cx, literal.span, "_", &mut applicability);
        span_lint_and_sugg(
            cx,
            NEEDLESS_PARENS_ON_RANGE_LITERALS,
            e.span,
            "needless parenthesis on range literals can be removed",
            "try",
            suggestion.to_string(),
            applicability,
        );
    }
}

impl EarlyLintPass for NeedlessParensOnRangeLiterals {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let ExprKind::Range(start, end, ..) = &expr.kind {
            if let Some(start) = start {
                check_for_parens(cx, start, true);
            }
            if let Some(end) = end {
                check_for_parens(cx, end, false);
            }
        }
    }
}

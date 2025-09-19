use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher;
use clippy_utils::source::{SpanRangeExt, snippet_with_applicability};

use rustc_ast::ast;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
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

fn check_for_parens(cx: &LateContext<'_>, e: &Expr<'_>, is_start: bool) {
    if is_start
        && let ExprKind::Lit(literal) = e.kind
        && let ast::LitKind::Float(_sym, ast::LitFloatType::Unsuffixed) = literal.node
    {
        // don't check floating point literals on the start expression of a range
        return;
    }
    if let ExprKind::Lit(literal) = e.kind
        // the indicator that parenthesis surround the literal is that the span of the expression and the literal differ
        && literal.span != e.span
        // inspect the source code of the expression for parenthesis
        && e.span.check_source_text(cx, |s| s.starts_with('(') && s.ends_with(')'))
    {
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

impl<'tcx> LateLintPass<'tcx> for NeedlessParensOnRangeLiterals {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let Some(higher::Range { start, end, .. }) = higher::Range::hir(expr) {
            if let Some(start) = start {
                check_for_parens(cx, start, true);
            }
            if let Some(end) = end {
                check_for_parens(cx, end, false);
            }
        }
    }
}

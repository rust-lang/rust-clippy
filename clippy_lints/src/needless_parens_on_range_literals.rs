use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher;
use clippy_utils::source::{SpanRangeExt, snippet_with_context};

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
    if !e.span.from_expansion()
        && let ExprKind::Lit(literal) = e.kind
        // the indicator that parenthesis surround the literal is that the span of the expression and the literal differ
        && literal.span != e.span
        // inspect the source code of the expression for parenthesis
        && e.span.check_source_text(cx, |s| s.starts_with('(') && s.ends_with(')'))
    {
        if is_start
            && let ast::LitKind::Float(_, ast::LitFloatType::Unsuffixed) = literal.node
            && literal.span.check_source_text(cx, |s| s.ends_with('.'))
        {
            // don't lint `(2.)..end`, since removing the parens would result in invalid syntax
            return;
        }

        let mut applicability = Applicability::MachineApplicable;
        let suggestion = snippet_with_context(cx, literal.span, e.span.ctxt(), "_", &mut applicability).0;
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
        if let Some(higher::Range { start, end, .. }) = higher::Range::hir(cx, expr) {
            if let Some(start) = start {
                check_for_parens(cx, start, true);
            }
            if let Some(end) = end {
                check_for_parens(cx, end, false);
            }
        }
    }
}

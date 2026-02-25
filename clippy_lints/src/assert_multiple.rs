use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::source::{snippet_indent, snippet_with_context};
use clippy_utils::sugg::strip_enclosing_paren;
use rustc_errors::Applicability;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::{SyntaxContext, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Looks for cases of `assert!(a == b && c == d)`
    ///
    /// ### Why is this bad?
    /// When such an assertion fails, there isn't any information about which particular sub-expression caused the failure.
    ///
    /// ### Example
    /// ```no_run
    /// let a = true;
    /// let b = true;
    /// let c = true;
    /// let d = true;
    /// assert!(a == b && c != d /* &&  ... */)
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let a = true;
    /// let b = true;
    /// let c = true;
    /// let d = true;
    /// assert_eq!(a, b);
    /// assert_ne!(c, d);
    /// /* ... */
    /// ```
    #[clippy::version = "1.97.0"]
    pub ASSERT_MULTIPLE,
    nursery,
    "Splitting an assert using `&&` into separate asserts makes it clearer which is failing."
}

declare_lint_pass!(AssertMultiple => [ASSERT_MULTIPLE]);

/// This visitor is a convenient place to hold the session context, as well as the collection of
/// replacement strings and the type of assert to use.
struct AssertVisitor<'tcx, 'v> {
    cx: &'v LateContext<'tcx>,
    suggests: Vec<String>,
    assert_string: &'static str,
    outerctxt: SyntaxContext,
}

impl<'tcx> Visitor<'tcx> for AssertVisitor<'tcx, '_> {
    fn visit_expr(&mut self, e: &'tcx Expr<'_>) {
        let mut app = Applicability::MaybeIncorrect;
        if let ExprKind::Binary(op, lhs, rhs) = e.kind {
            let lhs_name = snippet_with_context(self.cx, lhs.span, self.outerctxt, "..", &mut app).0;
            let rhs_name = snippet_with_context(self.cx, rhs.span, self.outerctxt, "..", &mut app).0;

            match op.node {
                BinOpKind::And => {
                    // For And, turn each of the rhs and lhs expressions into their own assert.
                    rustc_hir::intravisit::walk_expr(self, lhs);
                    rustc_hir::intravisit::walk_expr(self, rhs);
                },
                BinOpKind::Eq => {
                    let tmpstr = format!("{}_eq!({lhs_name}, {rhs_name});", self.assert_string);
                    self.suggests.push(tmpstr);
                },
                BinOpKind::Ne => {
                    let tmpstr = format!("{}_ne!({lhs_name}, {rhs_name});", self.assert_string);
                    self.suggests.push(tmpstr);
                },
                BinOpKind::Or | BinOpKind::Ge | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Lt => {
                    let snip = snippet_with_context(self.cx, e.span, self.outerctxt, "..", &mut app).0;
                    let stripped = strip_enclosing_paren(snip);
                    let tmpstr = format!("{}!({stripped});", self.assert_string);
                    self.suggests.push(tmpstr);
                },
                _ => {},
            }
        } else {
            let snip = snippet_with_context(self.cx, e.span, self.outerctxt, "..", &mut app).0;
            let tmpstr = format!("{}!({snip});", self.assert_string);
            self.suggests.push(tmpstr);
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for AssertMultiple {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, e)
            && let assert_string = match cx.tcx.get_diagnostic_name(macro_call.def_id) {
                Some(sym::assert_macro) => "assert",
                Some(sym::debug_assert_macro) => "debug_assert",
                _ => return,
            }
            && let Some((condition, _)) = find_assert_args(cx, e, macro_call.expn)
            && matches!(condition.kind, ExprKind::Binary(binop, _lhs, _rhs) if binop.node == BinOpKind::And)
        {
            // We only get here on assert/debug_assert macro calls whose arguments have an And expression
            // on the top of the tree.
            let mut am_visitor = AssertVisitor {
                cx,
                suggests: Vec::new(),
                assert_string,
                outerctxt: condition.span.ctxt(),
            };
            rustc_hir::intravisit::walk_expr(&mut am_visitor, condition);

            if !am_visitor.suggests.is_empty() {
                let indent = snippet_indent(cx, macro_call.span).unwrap_or_default();
                let jointext = format!("\n{indent}");
                let suggs = am_visitor.suggests.join(&jointext).trim_end_matches(';').to_string();
                span_lint_and_sugg(
                    cx,
                    ASSERT_MULTIPLE,
                    macro_call.span,
                    "multiple asserts combined into one",
                    "consider writing",
                    suggs,
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

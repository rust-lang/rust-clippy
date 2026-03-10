use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Looks for cases of assert!(a==b && c==d) and suggests alternative
    ///
    /// ### Why is this bad?
    /// It's hard to identify which test is failing
    /// ### Example
    /// ```no_run
    /// let a = true;
    /// let b = true;
    /// let c = true;
    /// let d = true;
    /// assert!(a==b && c!=d /* &&  ... */)
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = true;
    /// let b = true;
    /// let c = true;
    /// let d = true;
    /// assert_eq!(a, b);
    /// assert_ne!(c,d);
    /// /* ... */
    /// ```
    #[clippy::version = "1.95.0"]
    pub ASSERT_MULTIPLE,
    nursery,
    "Splitting an assert using '&&' into separate asserts makes it clearer which is failing."
}

declare_lint_pass!(AssertMultiple => [ASSERT_MULTIPLE]);

struct AssertVisitor<'tcx, 'v> {
    cx: &'v LateContext<'tcx>,
    suggests: Vec<String>,
    assert_string: &'v str,
}

impl<'tcx> Visitor<'tcx> for AssertVisitor<'tcx, '_> {
    fn visit_expr(&mut self, e: &'tcx Expr<'_>) {
        match e.kind {
            ExprKind::Binary(op, lhs, rhs) => match op.node {
                BinOpKind::And => {
                    rustc_hir::intravisit::walk_expr(self, lhs);
                    rustc_hir::intravisit::walk_expr(self, rhs);
                },
                BinOpKind::Or => {
                    let tmpstr = format!("{}!{};", self.assert_string, snippet(self.cx, e.span, ".."));
                    self.suggests.push(tmpstr);
                },
                _ => {
                    if let Some(x) = assert_from_op(self, op.node, *lhs, *rhs) {
                        self.suggests.push(x);
                    }
                },
            },
            ExprKind::Call(_call, _args) => {
                let tmptxt = snippet(self.cx, e.span, "..");
                let tmpassrt = format!("{}!({});", self.assert_string, tmptxt.trim_end_matches(';'));
                self.suggests.push(tmpassrt);
            },

            ExprKind::MethodCall(_path, expr, _args, span) => {
                let calltext = snippet(self.cx, expr.span, "..");

                let tmptxt = format!("{}.{};", &*calltext, snippet(self.cx, span, ".."));
                self.suggests.push(tmptxt);
            },
            ExprKind::Path(qpath) => {
                // this is a boolean variable
                let name = snippet(self.cx, qpath.span(), "_");
                let tmptxt = format!("{}!({name});", self.assert_string);
                self.suggests.push(tmptxt);
            },

            _ => {},
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
            && matches!(condition.kind, ExprKind::Binary(binop,_lhs,_rhs) if binop.node == BinOpKind::And)
        {
            let mut am_visitor = AssertVisitor {
                cx,
                suggests: Vec::new(),
                assert_string,
            };
            rustc_hir::intravisit::walk_expr(&mut am_visitor, condition);

            if !am_visitor.suggests.is_empty() {
                let suggs = am_visitor.suggests.join("\n    ").trim_end_matches(';').to_string();
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

fn assert_from_op(
    visitor: &mut AssertVisitor<'_, '_>,
    node: BinOpKind,
    lhs: Expr<'_>,
    rhs: Expr<'_>,
) -> Option<String> {
    let cx = visitor.cx;
    let lhs_name = snippet(cx, lhs.span, "_");
    let rhs_name = snippet(cx, rhs.span, "_");
    match node {
        BinOpKind::Eq => Some(format!("{}_eq!({lhs_name}, {rhs_name});", visitor.assert_string)),
        BinOpKind::Ne => Some(format!("{}_ne!({lhs_name}, {rhs_name});", visitor.assert_string)),
        BinOpKind::Ge | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Lt => Some(format!(
            "{}!({lhs_name} {} {rhs_name})",
            visitor.assert_string,
            node.as_str()
        )),
        _ => None,
    }
}

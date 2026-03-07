use clippy_utils::diagnostics::span_lint_and_then;
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
    /// Clearer assert output
    /// ### Example
    /// ```no_run
    /// assert!(a==b && c!=d && /* ... */)
    /// ```
    /// Use instead:
    /// ```no_run
    /// assert_eq!(a, b);
    /// assert_ne!(c,d);
    /// ...
    /// ```
    #[clippy::version = "1.95.0"]
    pub ASSERT_MULTIPLE,
    nursery,
    "default lint description"
}

declare_lint_pass!(AssertMultiple => [ASSERT_MULTIPLE]);

// the visitor needs a mutable reference to a vector that lives
// only for the duration of a single `check_expr` invocation.  we
// therefore introduce a separate lifetime `'v` for that borrow.
struct AssertVisitor<'tcx, 'v> {
    // the context reference only needs to live as long as the visitor,
    // which is represented by `'v` (the HIR lifetime `'tcx` refers to the
    // data inside the `LateContext`, not the borrow itself).
    cx: &'v LateContext<'tcx>,
    suggests: Vec<String>,
}

impl<'tcx, 'v> Visitor<'tcx> for AssertVisitor<'tcx, 'v> {
    fn visit_expr(&mut self, e: &'tcx Expr<'_>) {
        match e.kind {
            ExprKind::Binary(op, lhs, rhs) => {
                match op.node {
                    BinOpKind::And => {
                        eprintln!("are we here?");
                        rustc_hir::intravisit::walk_expr(self, lhs);
                        rustc_hir::intravisit::walk_expr(self, rhs);
                    },
                    _ => {
                        match assert_from_op(self.cx, op.node, *lhs, *rhs) {
                            Some(x) => self.suggests.push(x),
                            None => {},
                        };
                    },
                };
            },
            ExprKind::Call(_call, _args) => {
                let tmptxt = snippet(self.cx, e.span, "..");
                let tmpassrt = format!("assert!({});", tmptxt);
                self.suggests.push(tmpassrt);
            },

            ExprKind::MethodCall(_path, expr, _args, span) => {
                let calltext = snippet(self.cx, expr.span, "..");

                let tmptxt = format!("{}.{});", &*calltext, snippet(self.cx, span, ".."));
                self.suggests.push(tmptxt);
            },

            _ => {},
        };
    }
}

impl<'tcx> LateLintPass<'tcx> for AssertMultiple {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, e)
            && match cx.tcx.get_diagnostic_name(macro_call.def_id) {
                Some(sym::debug_assert_macro) => false,
                Some(sym::assert_macro) => true,
                _ => false,
            }
            && let Some((condition, _)) = find_assert_args(cx, e, macro_call.expn)
            && match condition.kind {
                ExprKind::Binary(binop, _lhs, _rhs) if matches!(binop.node, BinOpKind::And) => true,
                _ => false,
            }
        {
            let mut am_visitor = AssertVisitor {
                cx,
                suggests: Vec::new(),
            };
            rustc_hir::intravisit::walk_expr(&mut am_visitor, condition);

            if !am_visitor.suggests.is_empty() {
                // build the suggestion string outside of the closure to avoid
                // borrowing `suggests` while the diag closure runs
                let text = am_visitor.suggests.join("\n");
                let applicability = Applicability::MaybeIncorrect;
                span_lint_and_then(
                    cx,
                    ASSERT_MULTIPLE,
                    e.span,
                    "Multiple asserts combined into one",
                    move |diag| {
                        dbg!(&text);
                        diag.span_suggestion(e.span, "consider writing", text.clone(), applicability);
                    },
                );
            }
        }
    }
}

fn assert_from_op(cx: &LateContext<'_>, node: BinOpKind, lhs: Expr<'_>, rhs: Expr<'_>) -> Option<String> {
    let lhs_name = snippet(cx, lhs.span, "..");
    let rhs_name = snippet(cx, rhs.span, "..");
    match node {
        BinOpKind::Eq => Some(format!("assert_eq!({}, {});", lhs_name, rhs_name)),
        BinOpKind::Ne => Some(format!("assert_ne!({}, {});", lhs_name, rhs_name)),
        BinOpKind::Ge | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Lt => {
            Some(format!("assert!({} {} {})", lhs_name, node.as_str(), rhs_name))
        },
        _ => None,
    }
}

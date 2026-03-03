use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;
use std::borrow::Borrow;

declare_clippy_lint! {
    /// ### What it does
    ///  Looks for cases of assert!(a==b && c==d) and suggests alternative
    ///
    /// ### Why is this bad?
    ///   Clearer assert output
    /// ### Example
    /// ```no_run
    ///    assert!(a==b && c!=d || ...)
    /// ```
    /// Use instead:
    /// ```no_run
    ///   assert_eq!(a, b);
    ///   assert_ne!(c,d);
    ///   ...
    /// ```
    #[clippy::version = "1.95.0"]
    pub ASSERT_MULTIPLE,
    nursery,
    "default lint description"
}
declare_lint_pass!(AssertMultiple => [ASSERT_MULTIPLE]);

impl<'tcx> AssertMultiple {
    fn visit_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, suggest_asserts: &mut Vec<String>) {
        match e.kind {
            ExprKind::Binary(op, lhs, rhs) if matches!(op.node, BinOpKind::And | BinOpKind::Or) => {
                let _ = self.visit_expr(cx, lhs, suggest_asserts);
                let _ = self.visit_expr(cx, rhs, suggest_asserts);
            },
            ExprKind::Binary(op, lhs, rhs)
                if matches!(
                    op.node,
                    BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Gt | BinOpKind::Ge | BinOpKind::Lt | BinOpKind::Le
                ) =>
            {
                suggest_asserts.push(assert_from_op(&op.node, lhs, rhs));
            },

            ExprKind::Call(call, args) => {
                let tmptxt = assert_from_fncall(cx, call, args);
                suggest_asserts.push(tmptxt);
            },
            ExprKind::MethodCall(_path, expr, _args, span) => {
                let calltext = snippet(cx, span, "..");
                let mut tmptxt = "assert!(".to_string();

                if let ExprKind::Path(qpath) = expr.kind {
                    tmptxt += &name_from_qpath(&qpath);
                    tmptxt += ".";
                    tmptxt += &*calltext;
                    tmptxt += ");";
                    suggest_asserts.push(tmptxt);
                } else {
                    return;
                }
            },

            _ => {},
        };
    }
}

impl<'tcx> LateLintPass<'tcx> for AssertMultiple {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, e)
            && match cx.tcx.get_diagnostic_name(macro_call.def_id) {
                Some(sym::debug_assert_macro) => false,
                Some(sym::assert_macro) => true,
                _ => false,
            }
            && let Some((condition, _)) = find_assert_args(cx, e, macro_call.expn)
            && match condition.kind {
                ExprKind::Binary(binop, _lhs, _rhs) if matches!(binop.node, BinOpKind::And | BinOpKind::Or) => true,
                _ => false,
            }
        {
            let mut suggest_asserts: Vec<String> = Vec::new();
            // first node of assert is BinOpKind::Not, skip it);
            self.visit_expr(cx, condition, &mut suggest_asserts);
            if suggest_asserts.len() != 0 {
                let applicability = Applicability::MaybeIncorrect;
                span_lint_and_then(
                    cx,
                    ASSERT_MULTIPLE,
                    e.span,
                    "Multiple asserts combined into one",
                    |diag| {
                        let text = suggest_asserts.join("\n");
                        dbg!(&text);
                        diag.span_suggestion(e.span, "consider writing", "my text", applicability);
                    },
                );
            }
        }
    }
}

fn name_from_qpath(qpath: &QPath<'_>) -> String {
    let mut retstr: String = "".to_string();
    let QPath::Resolved(_, path) = qpath else { return retstr };
    let seg_cnt = path.segments.len() - 1;
    let segiter = path.segments.iter().enumerate();
    for (idex, segment) in segiter {
        retstr.push_str(segment.ident.name.as_str());
        if idex != seg_cnt {
            retstr.push_str("::");
        };
    }
    retstr
}

fn assert_from_op(node: &BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>) -> String {
    let mut lhs_name: String = "".to_string();
    let mut rhs_name: String = "".to_string();

    if let ExprKind::Path(qpath) = lhs.kind {
        lhs_name = name_from_qpath(&qpath);
    };

    if let ExprKind::Path(qpath) = rhs.kind {
        rhs_name = name_from_qpath(&qpath);
    };
    match node {
        BinOpKind::Eq => {
            format!("assert_eq!({}, {});", lhs_name, rhs_name)
        },
        BinOpKind::Ne => {
            format!("assert_ne!({},{});", lhs_name, rhs_name)
        },
        BinOpKind::Ge | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Lt => {
            format!("assert!({} {} {})", lhs_name, node.as_str(), rhs_name)
        },
        _ => {
            panic!("not handled")
        },
    }
}

fn assert_from_fncall(cx: &LateContext<'_>, call: &Expr<'_>, args: &[Expr<'_>]) -> String {
    let mut retstr = "assert!(".to_string();

    if let ExprKind::Path(qpath) = call.kind {
        let snip = snippet(cx, qpath.span(), "..");
        retstr.push_str(snip.borrow());
    }
    retstr.push_str("(");

    if args.len() > 0 {
        let arglen = args.len() - 1;
        for (idx, arg) in args.iter().enumerate() {
            let lit_text = snippet(cx, arg.span, "..");
            retstr.push_str(&*lit_text);
            if idx != arglen {
                retstr.push_str(",");
            }
        }
    }

    retstr.push_str("));");
    retstr
}

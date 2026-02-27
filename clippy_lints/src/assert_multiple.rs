use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::source::snippet;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

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
                eprintln!("found call");
                let tmptxt = assert_from_fncall(cx, call, args);
                dbg!(&tmptxt);
                suggest_asserts.push(tmptxt);
            },

            _ => {},
        };
    }
}

impl<'tcx> LateLintPass<'tcx> for AssertMultiple {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        let Some(macro_call) = root_macro_call_first_node(cx, e) else {
            return;
        };
        let _ = match cx.tcx.get_diagnostic_name(macro_call.def_id) {
            Some(sym::debug_assert_macro) => return,
            Some(sym::assert_macro) => false,
            _ => return,
        };
        let condition = match find_assert_args(cx, e, macro_call.expn) {
            Some((cn, _)) => cn,
            _ => return,
        };
        //          dbg!(condition);
        //        let (lhs,rhs) = match condition.kind {
        //            ExprKind::Binary(op, lhs, rhs) if matches!(op.node, BinOpKind::And) => (lhs,rhs),
        //            _ => return,
        //        };
        let mut suggest_asserts: Vec<String> = Vec::new();
        dbg!(condition);
        self.visit_expr(cx, condition, &mut suggest_asserts);
        dbg!(suggest_asserts);
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
        let callname = name_from_qpath(&qpath);
        retstr.push_str(callname.as_str());
    }
    retstr.push_str("(");

    if args.len() > 0 {
        let arglen = args.len() - 1;
        for (idx, arg) in args.iter().enumerate() {
            let lit_text = snippet(cx, arg.span, "..");
            retstr.push_str(&*lit_text);
            dbg!(&retstr);
            if idx != arglen {
                retstr.push_str(",");
            }
        }
    }

    retstr.push_str("));");
    retstr
}

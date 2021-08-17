use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use rustc_errors::Applicability;
use rustc_hir::UnOp;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual deref in function parameter.
    ///
    /// ### Why is this bad?
    /// There is no need to deref manually. Compiler will auto deref.
    ///
    /// ### Known problems
    /// Complicate type is not handled. For example `foo(&*****(&&T))`.
    ///
    /// ### Example
    /// ```rust
    /// fn foo(_: &str) {}
    /// let pf = &String::new();
    /// // Bad
    /// foo(&**pf);
    /// foo(&*String::new());
    ///
    /// // Good
    /// foo(pf);
    /// foo(&String::new());
    /// ```
    pub NEEDLESS_DEREF,
    pedantic,
    "remove needless deref"
}

declare_lint_pass!(NeedlessDeref => [NEEDLESS_DEREF]);

impl LateLintPass<'_> for NeedlessDeref {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        match e.kind {
            ExprKind::Call(fn_expr, arguments) => {
                if let ExprKind::Path(..) = fn_expr.kind {
                    // contain no generic parameter
                    if cx.typeck_results().node_substs_opt(fn_expr.hir_id).is_none() {
                        check_arguments(cx, arguments, cx.typeck_results().expr_ty(fn_expr));
                    }
                }
            },
            ExprKind::MethodCall(path, _, arguments, _) => {
                // contain no generic parameter
                if path.args.is_none() && cx.typeck_results().node_substs_opt(e.hir_id).is_none() {
                    let def_id = cx.typeck_results().type_dependent_def_id(e.hir_id).unwrap();
                    let method_type = cx.tcx.type_of(def_id);
                    check_arguments(cx, arguments, method_type);
                }
            },
            _ => (),
        }
    }
}

fn check_arguments<'tcx>(cx: &LateContext<'tcx>, arguments: &[Expr<'_>], type_definition: Ty<'tcx>) {
    match type_definition.kind() {
        ty::FnDef(..) | ty::FnPtr(_) => {
            for argument in arguments.iter() {
                // a: &T
                // foo(&** a) -> foo(a)
                if_chain! {
                    if let ExprKind::AddrOf(_, _, child1) = argument.kind ;
                    if let ExprKind::Unary(UnOp::Deref, child2) = child1.kind ;
                    if let ExprKind::Unary(UnOp::Deref, child3) = child2.kind ;
                    if !matches!(child3.kind,ExprKind::Unary(UnOp::Deref, ..) );
                    let ty = cx.typeck_results().expr_ty(child3);
                    if matches!(ty.kind(),ty::Ref(..));
                    then{
                        span_lint_and_sugg(
                            cx,
                            NEEDLESS_DEREF,
                            argument.span,
                            "needless explicit deref in function parameter",
                            "try remove the `&**` and just keep",
                            snippet_opt(cx, child3.span).unwrap(),
                            Applicability::MachineApplicable,
                        );
                    }
                }

                // a: T
                // foo(&*a) -> foo(&a)
                if_chain! {
                    if let ExprKind::AddrOf(_, _, child1) = argument.kind ;
                    if let ExprKind::Unary(UnOp::Deref, child2) = child1.kind ;
                    if !matches!(child2.kind,ExprKind::Unary(UnOp::Deref, ..) );
                    let ty = cx.typeck_results().expr_ty(child2);
                    if !matches!(ty.kind(),ty::Ref(..));
                    then{
                        span_lint_and_sugg(
                            cx,
                            NEEDLESS_DEREF,
                            argument.span,
                            "needless explicit deref in function parameter",
                            "you can replace this with",
                            ("&".to_owned()+&snippet_opt(cx, child2.span).unwrap()).clone(),
                            Applicability::MachineApplicable,
                        );
                    }
                }
            }
        },
        _ => (),
    }
}

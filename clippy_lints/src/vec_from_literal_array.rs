use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::AssocContainer;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for vec creation by calling `.to_vec()` on a literal array.
    ///
    /// ### Why is this bad?
    ///
    /// It is clearer to use the `vec!` macro.
    ///
    /// ### Example
    /// ```no_run
    /// let value = [1, 2, 3].to_vec();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let value = vec![1, 2, 3];
    /// ```
    #[clippy::version = "1.99.0"]
    pub VEC_FROM_LITERAL_ARRAY,
    pedantic,
    "creating vec with to_vec() on literal array"
}

declare_lint_pass!(VecFromLiteralArray => [VEC_FROM_LITERAL_ARRAY]);

impl LateLintPass<'_> for VecFromLiteralArray {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if let ExprKind::MethodCall(method_name, receiver, args, _) = expr.kind
            && method_name.ident.name == sym::to_vec
            && let ExprKind::Array(_) = receiver.kind
            && args.is_empty()
        {
            // Make sure that this to_vec() doesn't come from a trait
            if let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
                && let assoc = cx.tcx.associated_item(method_def_id)
                && assoc.container != AssocContainer::InherentImpl
            {
                return;
            }
            let mut app = Applicability::MachineApplicable;
            let (recv, _) = snippet_with_context(cx, receiver.span, expr.span.ctxt(), "_", &mut app);
            span_lint_and_sugg(
                cx,
                VEC_FROM_LITERAL_ARRAY,
                expr.span,
                "creating a vec with a literal array",
                "try",
                format!("vec!{recv}"),
                app,
            );
        }
    }
}

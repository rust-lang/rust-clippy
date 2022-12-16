use rustc_hir::*;
use rustc_ast as ast;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.67.0"]
    pub OPTION_IS_SOME_AND,
    nursery,
    "default lint description"
}
declare_lint_pass!(OptionIsSomeAnd => [OPTION_IS_SOME_AND]);

impl<'tcx> LateLintPass<'tcx> for OptionIsSomeAnd {
    fn check_fn(
        &mut self,
        _: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        fn_decl: &'tcx FnDecl<'tcx>,
        fn_body: &'tcx Body<'tcx>,
        span: rustc_span::Span,
        _: HirId
        ) {
            println!("Working! {:?} {:?} {:?}\n", span, fn_decl, fn_body);
            // if uses_map_unwrap_or_false() {
            //     println!("Actually working!");
            // }
        }

    // fn uses_map_unwrap_or_false() {
    //     if let ExprKind::MethodCall(method_name, receiver, args, _) = expr.kind
    //         && method_name.ident.as_str() == "unwrap_or"
    //         && let ExprKind::MethodCall(method_name1, receiver1, args1, _) = receiver.kind
    //         && method_name1.ident.as_str() == "map"
    //         && args.len() == 1
    //         && let ExprKind::Lit(ref lit) = args[0].kind
    //         && let ast::LitKind::Bool(false) = lit.node {
            
    //         true
    //     }

    //     false
    // }
}

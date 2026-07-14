use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::visitors::is_local_used;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::middle::privacy::Level;
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// fn foo(a: i32, _b: i32) {
    ///     println!("{a}");
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(a: i32) {
    ///     println!("{a}");
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
    pedantic,
    "default lint description"
}
declare_lint_pass!(UnusedUnderscorePrefixedArgument => [UNUSED_UNDERSCORE_PREFIXED_ARGUMENT]);

impl<'tcx> LateLintPass<'tcx> for UnusedUnderscorePrefixedArgument {
    fn check_fn(
        &mut self,
        ctx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        _fn_decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        def_id: LocalDefId,
    ) {
        match fn_kind {
            FnKind::ItemFn(..) | FnKind::Method(..) => {
                if ctx.effective_visibilities.is_exported(def_id) {
                    return;
                }
                for param in body.params {
                    if let PatKind::Binding(_, hir_id, name, _) = param.pat.kind
                        && name.as_str().starts_with('_')
                        && !is_local_used(ctx, body, hir_id)
                    {
                        span_lint_and_help(
                            ctx,
                            UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
                            param.span,
                            "argument with an underscore prefix which is unused",
                            None,
                            "consider removing the parameter",
                        );
                    }
                }
            },
            _ => {}
        }
    }
}

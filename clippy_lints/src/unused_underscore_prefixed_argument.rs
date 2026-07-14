use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_trait_impl_item;
use clippy_utils::visitors::is_local_used;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for function parameters that are prefixed with an underscore but are never used.
    ///
    /// This only applies to functions that are not effectively exported (i.e. not reachable from
    /// outside the crate), such as private functions, functions in a private struct's impl, or
    /// private functions in a public struct's impl.
    ///
    /// ### Why is this bad?
    ///
    /// An underscore prefix only silences the unused-variable warning. If the parameter is
    /// genuinely never used, it is dead weight and can usually be removed entirely.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo(a: i32, _b: i32) {
    ///     println!("{a}");
    /// }
    ///
    /// struct S;
    /// impl S {
    ///     fn bar(&self, _unused: i32) {}
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(a: i32) {
    ///     println!("{a}");
    /// }
    ///
    /// struct S;
    /// impl S {
    ///     fn bar(&self) {}
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
    pedantic,
    "function parameter prefixed with `_` that is never used"
}

declare_lint_pass!(UnusedUnderscorePrefixedArgument => [
    UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
]);

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
                let hir_id = ctx.tcx.local_def_id_to_hir_id(def_id);
                if is_trait_impl_item(ctx, hir_id) {
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
            _ => {},
        }
    }
}

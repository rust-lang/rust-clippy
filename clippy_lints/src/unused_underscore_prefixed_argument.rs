use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::is_trait_impl_item;
use clippy_utils::visitors::is_local_used;
use rustc_abi::ExternAbi;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{LocalDefId, LocalDefIdSet};
use rustc_hir::intravisit::FnKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
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

impl_lint_pass!(UnusedUnderscorePrefixedArgument => [
    UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
]);

struct Candidate {
    fn_def_id: LocalDefId,
    param_span: Span,
}

#[derive(Default)]
pub struct UnusedUnderscorePrefixedArgument {
    fns_as_value: LocalDefIdSet,
    candidates: Vec<Candidate>,
}

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
        if let Some(header) = fn_kind.header() {
            if header.abi != ExternAbi::Rust {
                return;
            }
        }
        if let FnKind::ItemFn(..) | FnKind::Method(..) = fn_kind {
            {
                if ctx.effective_visibilities.is_exported(def_id) {
                    return;
                }
                if is_trait_impl_item(ctx, ctx.tcx.local_def_id_to_hir_id(def_id)) {
                    return;
                }
                for param in body.params {
                    if let PatKind::Binding(_, hir_id, name, _) = param.pat.kind
                        && name.as_str().starts_with('_')
                        && !is_local_used(ctx, body, hir_id)
                    {
                        self.candidates.push(Candidate {
                            fn_def_id: def_id,
                            param_span: param.span,
                        });
                    }
                }
            }
        }
    }

    fn check_path(&mut self, ctx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        if let Some(def_id) = path.res.opt_def_id()
            && let Some(local_def_id) = def_id.as_local()
            && matches!(ctx.tcx.def_kind(local_def_id), DefKind::Fn | DefKind::AssocFn)
            && let parent = ctx.tcx.parent_hir_node(hir_id)
            && !matches!(
                parent,
                Node::Expr(Expr {
                    kind: ExprKind::Call(Expr { span, .. }, _),
                    ..
                }) if *span == path.span
            )
        {
            self.fns_as_value.insert(local_def_id);
        }
    }

    fn check_crate_post(&mut self, ctx: &LateContext<'tcx>) {
        let warnables: Vec<&Candidate> = self
            .candidates
            .iter()
            .filter(|candidate| !self.fns_as_value.contains(&candidate.fn_def_id))
            .collect();

        for warnable in warnables {
            span_lint_hir_and_then(
                ctx,
                UNUSED_UNDERSCORE_PREFIXED_ARGUMENT,
                ctx.tcx.local_def_id_to_hir_id(warnable.fn_def_id),
                warnable.param_span,
                "argument with an underscore prefix which is unused",
                |diag| {
                    diag.help("consider removing the parameter");
                },
            );
        }
    }
}

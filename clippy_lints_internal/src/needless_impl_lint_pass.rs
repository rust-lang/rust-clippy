use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_expn_of, sym};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Item, ItemKind, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_tool_lint! {
    /// ### What it does
    /// Checks for usage of `impl_lint_pass!` on lint pass structs without fields, and suggests
    /// to use `declare_lint_pass!` instead.
    ///
    /// ### Why is this bad?
    /// Using `impl_lint_pass!` is only necessary when the lint pass struct needs to have fields.
    /// Without them, `declare_lint_pass!` can be used for more concise code.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use rustc_session::impl_lint_pass;
    ///
    /// struct LintPassWithoutFields;
    ///
    /// impl_lint_pass!(LintPassWithoutFields => []);
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use rustc_session::declare_lint_pass;
    ///
    /// declare_lint_pass!(LintPassWithoutFields => [SOME_LINT]);
    /// ```
    ///
    /// ### Known problems
    /// FN if `impl_lint_pass!` is used in a qualified way, e.g. as `rustc_session::impl_lint_pass!`
    pub clippy::NEEDLESS_IMPL_LINT_PASS,
    Warn,
    "using `impl_lint_pass!` for a lint pass struct without fields",
    report_in_external_macro: false
}

declare_lint_pass!(NeedlessImplLintPass => [NEEDLESS_IMPL_LINT_PASS]);

impl<'tcx> LateLintPass<'tcx> for NeedlessImplLintPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let Some(impl_lint_pass_span) = is_expn_of(item.span, sym::impl_lint_pass)
            // `declare_lint_pass!` itself calls `impl_lint_pass!` -- make sure that we're not inside the former
            && is_expn_of(impl_lint_pass_span, sym::declare_lint_pass).is_none()
            // `impl_lint_pass!` contains two impls, one of which is an inherent impl
            // on the lint pass struct -- that's the one we'll look for
            && let ItemKind::Impl(impl_) = item.kind
            && impl_.of_trait.is_none()
            && let TyKind::Path(QPath::Resolved(None, path)) = impl_.self_ty.kind
            && let Res::Def(DefKind::Struct, lint_pass_struct) = path.res
            // check that the lint pass struct doesn't have any fields
            && cx.tcx.adt_def(lint_pass_struct).all_fields().next().is_none()
        {
            span_lint_and_then(
                cx,
                NEEDLESS_IMPL_LINT_PASS,
                impl_lint_pass_span,
                "`impl_lint_pass!` on a lint pass struct without fields",
                |diag| {
                    let Some(lint_pass_struct) = lint_pass_struct.as_local() else {
                        // Shouldn't really happen, as lint passes are basically always impl'd where they're defined
                        return;
                    };
                    let lint_pass_decl = cx.tcx.hir_node_by_def_id(lint_pass_struct).expect_item();
                    diag.span_label(lint_pass_decl.span, "struct defined here");
                    let remove_lint_pass_span = clippy_utils::sugg::remove_item_span(cx, lint_pass_decl.span);
                    let sugg = vec![
                        (remove_lint_pass_span, String::new()),
                        // Cut out the `impl` from `impl_lint_pass!` and replace it with `declare`
                        // -- sidesteps the annoyance of creating the snippets
                        (impl_lint_pass_span.split_at(4).0, "declare".to_string()),
                    ];
                    diag.multipart_suggestion_verbose(
                        "use `declare_lint_pass!`",
                        sugg,
                        Applicability::MaybeIncorrect, // because of the import
                    );
                    diag.help("You may need to import it first: `use rustc_session::declare_lint_pass;`");
                },
            );
        }
    }
}

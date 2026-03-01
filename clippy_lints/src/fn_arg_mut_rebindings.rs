use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::get_enclosing_block;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{
    BindingMode, Body, BodyId, ExprKind, ImplItem, ImplItemImplKind, ImplItemKind, Item, ItemKind, LetStmt, OwnerNode,
    Param, Pat, PatKind, QPath, TraitFn, TraitItem, TraitItemKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for function arguments declared as not mutable and later rebound as mutable.
    ///
    /// ### Why is this bad?
    /// It can be easily improved by just declaring the function argument as mutable and
    /// removing the unnecessary assignment.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo_bad(bar: Vec<i32>) -> Vec<i32> {
    ///     let mut bar = bar;
    ///     bar.push(42);
    ///     bar
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn foo(mut bar: Vec<i32>) -> Vec<i32> {
    ///     bar.push(42);
    ///     bar
    /// }
    /// ```
    #[clippy::version = "1.95.0"]
    pub FN_ARG_MUT_REBINDINGS,
    style,
    "non-mutable function argument rebound as mutable"
}
declare_lint_pass!(FnArgMutRebindings => [FN_ARG_MUT_REBINDINGS]);

impl LateLintPass<'_> for FnArgMutRebindings {
    fn check_local(&mut self, cx: &LateContext<'_>, st: &'_ LetStmt<'_>) {
        if !st.span.in_external_macro(cx.tcx.sess.source_map())

            // check let statement binds as mutable
            && let PatKind::Binding(BindingMode::MUT, _, ident, None) = st.pat.kind
            && let Some(init) = st.init

            // check let statement binds to variable with same name
            && let ExprKind::Path(QPath::Resolved(_, path)) = init.kind
            && path.segments.len() == 1
            && path.segments[0].ident == ident
            && let Res::Local(id) = path.res

            // check let statement scope is whole function body
            && let Some((_, owner)) = cx.tcx.hir_parent_owner_iter(st.hir_id).next()
            && let Some(body_id) = fn_body_id(&owner)
            && let &Body { params, value } = cx.tcx.hir_body(body_id)
            && let ExprKind::Block(fn_block, _) = value.kind
            && let Some(st_block) = get_enclosing_block(cx, st.hir_id)
            && fn_block.hir_id == st_block.hir_id

            // check param declares as immutable
            && let Some(&Param {
                span: pat_span,
                pat:
                    Pat {
                        kind: PatKind::Binding(BindingMode::NONE, ..),
                        ..
                    },
                ..
            }) = params.iter().find(|p| p.pat.hir_id == id)
        {
            span_lint_and_then(
                cx,
                FN_ARG_MUT_REBINDINGS,
                pat_span,
                format!(
                    "argument `{}` is declared as not mutable, and later rebound as mutable",
                    ident.name
                ),
                |diag| {
                    diag.span_suggestion(
                        pat_span,
                        "consider just declaring as mutable",
                        format!("mut {}", snippet(cx, pat_span, "_")),
                        Applicability::MaybeIncorrect,
                    );
                    diag.span_help(st.span, "and remove this");
                },
            );
        }
    }
}

fn fn_body_id(node: &OwnerNode<'_>) -> Option<BodyId> {
    match node {
        OwnerNode::Item(Item {
            kind: ItemKind::Fn { body: body_id, .. },
            ..
        })
        | OwnerNode::ImplItem(ImplItem {
            kind: ImplItemKind::Fn(_, body_id),
            // avoid false-positive: trait-fn can come from external crate
            impl_kind: ImplItemImplKind::Inherent { .. },
            ..
        })
        | OwnerNode::TraitItem(TraitItem {
            kind: TraitItemKind::Fn(_, TraitFn::Provided(body_id)),
            ..
        }) => Some(*body_id),
        _ => None,
    }
}

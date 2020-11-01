use crate::utils::{snippet, span_lint_and_then};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{
    BindingAnnotation, ExprKind, ImplItem, ImplItemKind, Item, ItemKind, Local, Node, PatKind, QPath, TraitFn,
    TraitItem, TraitItemKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Checks for function arguments declared as not mutable and
    /// later rebound as mutable.
    ///
    /// **Why is this bad?** It can be easily improved by just declaring the function
    /// argument as mutable and removing the unnecessary assignment.
    ///
    /// **Known problems:** The function argument might have been shadowed by another
    /// value before the assignment.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// fn foo_bad(bar: Vec<i32>) -> Vec<i32> {
    ///     let mut bar = bar;
    ///     bar.push(42);
    ///     bar
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// fn foo(mut bar: Vec<i32>) -> Vec<i32> {
    ///     bar.push(42);
    ///     bar
    /// }
    /// ```
    pub REBIND_FN_ARG_AS_MUT,
    style,
    "non-mutable function argument rebound as mutable"
}

declare_lint_pass!(RebindFnArgAsMut => [REBIND_FN_ARG_AS_MUT]);

impl LateLintPass<'_> for RebindFnArgAsMut {
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &Local<'tcx>) {
        if_chain! {
            if !in_external_macro(cx.tcx.sess, local.span);

            // LHS
            if let PatKind::Binding(BindingAnnotation::Mutable, _, name, None) = local.pat.kind;

            // RHS
            if let Some(init) = local.init;
            if let ExprKind::Path(QPath::Resolved(_, path)) = init.kind;
            if path.segments.len() == 1;
            if path.segments[0].ident == name;

            if let rustc_hir::def::Res::Local(id) = path.res;

            // Fn
            let parent_id = cx.tcx.hir().get_parent_item(id);

            if let Node::Item(&Item { kind: ItemKind::Fn(_, _, body_id), .. })
                | Node::ImplItem(&ImplItem { kind: ImplItemKind::Fn(_, body_id), .. })
                | Node::TraitItem(&TraitItem { kind: TraitItemKind::Fn(_, TraitFn::Provided(body_id)), .. })
                = cx.tcx.hir().get(parent_id);

            let body = cx.tcx.hir().body(body_id);
            if let Some(param) = body.params.iter().find(|param| param.pat.hir_id == id);
            if let PatKind::Binding(BindingAnnotation::Unannotated, ..) = param.pat.kind;

            then {
                span_lint_and_then(
                    cx,
                    REBIND_FN_ARG_AS_MUT,
                    param.pat.span,
                    &format!("argument `{}` is declared as not mutable, and later rebound as mutable", name),
                    |diag| {
                        diag.span_suggestion(
                            param.pat.span,
                            "consider just declaring as mutable",
                            format!("mut {}", snippet(cx, param.pat.span, "_")),
                            Applicability::MaybeIncorrect,
                        );
                        diag.span_help(local.span, "and remove this");
                    },
                );
            }
        }
    }
}

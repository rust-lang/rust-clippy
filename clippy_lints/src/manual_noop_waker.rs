use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{paths, sym};
use rustc_hir::{Body, ExprKind, ImplItemKind, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual implementations of `std::task::Wake` that are empty.
    ///
    /// ### Why is this bad?
    /// `Waker::noop()` provides a more performant and cleaner way to create a
    /// waker that does nothing, avoiding unnecessary `Arc` allocations and
    /// reference count increments.
    ///
    /// ### Example
    /// ```rust
    /// # use std::sync::Arc;
    /// # use std::task::Wake;
    /// struct MyWaker;
    /// impl Wake for MyWaker {
    ///     fn wake(self: Arc<Self>) {}
    ///     fn wake_by_ref(self: &Arc<Self>) {}
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// use std::task::Waker;
    /// let waker = Waker::noop();
    /// ```
    #[clippy::version = "1.96.0"]
    pub MANUAL_NOOP_WAKER,
    complexity,
    "manual implementations of noop wakers can be simplified using Waker::noop()"
}

declare_lint_pass!(ManualNoopWaker => [MANUAL_NOOP_WAKER]);

impl<'tcx> LateLintPass<'tcx> for ManualNoopWaker {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Impl(imp) = item.kind
            && let Some(trait_ref) = imp.of_trait
            && let Some(trait_id) = trait_ref.trait_ref.trait_def_id()
            && paths::WAKE.matches(cx, trait_id)
        {
            let mut has_wake = false;
            let mut has_wake_by_ref = false;
            let mut wake_empty = false;
            let mut wake_by_ref_empty = false;

            for impl_item_ref in imp.items {
                let def_id = impl_item_ref.owner_id.def_id;
                let impl_item = cx.tcx.hir_node_by_def_id(def_id).expect_impl_item();

                let method_name = impl_item.ident.name;

                if method_name == sym::wake {
                    has_wake = true;
                    if let ImplItemKind::Fn(_sig, body_id) = &impl_item.kind {
                        let body = cx.tcx.hir_body(*body_id);
                        if is_body_empty(body) {
                            wake_empty = true;
                        }
                    }
                } else if method_name == sym::wake_by_ref {
                    has_wake_by_ref = true;
                    if let ImplItemKind::Fn(_sig, body_id) = &impl_item.kind {
                        let body = cx.tcx.hir_body(*body_id);
                        if is_body_empty(body) {
                            wake_by_ref_empty = true;
                        }
                    }
                }
            }

            let is_noop_waker = if has_wake && wake_empty {
                if has_wake_by_ref { wake_by_ref_empty } else { true }
            } else {
                false
            };

            if is_noop_waker {
                span_lint_and_help(
                    cx,
                    MANUAL_NOOP_WAKER,
                    trait_ref.trait_ref.path.span,
                    "manual implementation of a no-op waker",
                    None,
                    "use `std::task::Waker::noop()` instead",
                );
            }
        }
    }
}

fn is_body_empty(body: &Body<'_>) -> bool {
    if let ExprKind::Block(block, _) = body.value.kind {
        return block.stmts.is_empty() && block.expr.is_none();
    }
    false
}

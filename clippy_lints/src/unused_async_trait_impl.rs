use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::usage::is_todo_unimplemented_stub;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{
    Body, Closure, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, ImplItem, ImplItemKind, IsAsync,
    YieldSource,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for trait function implementations that are declared `async` but have no `.await`s inside of them.
    ///
    /// ### Why is this bad?
    /// Async functions with no async code create computational overhead.
    /// Even though the trait requires the function to return a future,
    /// returning a `core::future::ready` with the result is more efficient
    /// as it reduces the number of states in the Future state machine by at least one.
    ///
    /// Note that the behaviour is slightly different when using core::future::ready,
    /// as the value is computed immediately and stored in a future for later retrieval at the first (and only valid) call to `poll`.
    /// An `async` block generates code that completely defers the computation of this value until the Future is polled.
    ///
    /// ### Example
    /// ```no_run
    /// trait AsyncTrait {
    ///     async fn get_random_number() -> i64;
    /// }
    ///
    /// impl AsyncTrait for () {
    ///     async fn get_random_number() -> i64 {
    ///         4 // Chosen by fair dice roll. Guaranteed to be random.
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// trait AsyncTrait {
    ///     async fn get_random_number() -> i64;
    /// }
    ///
    /// impl AsyncTrait for () {
    ///     fn get_random_number() -> impl Future<Output = i64> {
    ///         core::future::ready(4) // Chosen by fair dice roll. Guaranteed to be random.
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.94.0"]
    pub UNUSED_ASYNC_TRAIT_IMPL,
    pedantic,
    "finds async trait impl functions with no await statements"
}

declare_lint_pass!(UnusedAsyncTraitImpl => [UNUSED_ASYNC_TRAIT_IMPL]);

struct AsyncFnVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    found_await: bool,
    async_depth: usize,
}

impl<'tcx> Visitor<'tcx> for AsyncFnVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
        if let ExprKind::Yield(_, YieldSource::Await { .. }) = ex.kind
            && self.async_depth == 1
        {
            self.found_await = true;
        }

        let is_async_block = matches!(
            ex.kind,
            ExprKind::Closure(Closure {
                kind: ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _)),
                ..
            })
        );

        if is_async_block {
            self.async_depth += 1;
        }

        walk_expr(self, ex);

        if is_async_block {
            self.async_depth -= 1;
        }
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}

impl<'tcx> LateLintPass<'tcx> for UnusedAsyncTraitImpl {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'_>) {
        if let ImplItemKind::Fn(ref sig, body_id) = impl_item.kind
            && let IsAsync::Async(async_span) = sig.header.asyncness
            && let body = cx.tcx.hir_body(body_id)
            && !async_fn_contains_todo_unimplemented_macro(cx, body)
        {
            let mut visitor = AsyncFnVisitor {
                cx,
                found_await: false,
                async_depth: 0,
            };
            visitor.visit_nested_body(body_id);

            if !visitor.found_await {
                span_lint_hir_and_then(
                    cx,
                    UNUSED_ASYNC_TRAIT_IMPL,
                    cx.tcx.local_def_id_to_hir_id(impl_item.owner_id.def_id),
                    sig.span,
                    "unused `async` for async trait impl function with no await statements",
                    |diag| {
                        if let Some(output_src) = sig.decl.output.span().get_source_text(cx)
                            && let Some(body_src) = body.value.span.get_source_text(cx)
                        {
                            let output_str = output_src.as_str();
                            let body_str = body_src.as_str();

                            let sugg = vec![
                                (async_span, String::new()),
                                (sig.decl.output.span(), format!("impl Future<Output = {output_str}>")),
                                (body.value.span, format!("{{ core::future::ready({body_str}) }}")),
                            ];

                            diag.help("a Future can be constructed from the return value with `core::future::ready`");
                            diag.multipart_suggestion(
                                    format!("consider removing the `async` from this function and returning `impl Future<Output = {output_str}>` instead"),
                                    sugg,
                                    Applicability::MachineApplicable
                                );
                        }
                    },
                );
            }
        }
    }
}

fn async_fn_contains_todo_unimplemented_macro(cx: &LateContext<'_>, body: &Body<'_>) -> bool {
    if let ExprKind::Closure(closure) = body.value.kind
        && let ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _)) = closure.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let ExprKind::Block(block, _) = body.value.kind
        && block.stmts.is_empty()
        && let Some(expr) = block.expr
        && let ExprKind::DropTemps(inner) = expr.kind
    {
        return is_todo_unimplemented_stub(cx, inner);
    }

    false
}

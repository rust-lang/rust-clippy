use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::source::{
    HasSession, indent_of, reindent_multiline, snippet_block_with_applicability, snippet_with_applicability,
};
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
    /// Checks for trait method implementations that are declared `async` but have no `.await`s inside of them.
    ///
    /// ### Why is this bad?
    /// Async functions with no async code create computational overhead.
    /// Even though the trait requires the method to return a future,
    /// returning a `core::future::ready` with the result is more efficient
    /// as it reduces the number of states in the Future state machine by at least one.
    ///
    /// Note that the behaviour is slightly different when using `core::future::ready`,
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

            if !visitor.found_await
                && let Some(builtin_crate) = clippy_utils::std_or_core(cx)
            {
                span_lint_hir_and_then(
                    cx,
                    UNUSED_ASYNC_TRAIT_IMPL,
                    cx.tcx.local_def_id_to_hir_id(impl_item.owner_id.def_id),
                    impl_item.span,
                    "unused `async` for async trait impl function with no await statements",
                    |diag| {
                        let mut app = Applicability::MachineApplicable;

                        let async_span = cx.sess().source_map().span_extend_while_whitespace(async_span);

                        let signature_snippet = snippet_with_applicability(cx, sig.decl.output.span(), "_", &mut app);

                        // Fetch body snippet and truncate excess indentation. Like this:
                        // {
                        //     4
                        // }
                        let body_snippet = snippet_block_with_applicability(cx, body.value.span, "_", None, &mut app);
                        // Wrap body snippet in `std::future::ready(...)` and indent everything by one level, like this:
                        //     core::future::ready({
                        //         4
                        //     })
                        let new_body_inner_snippet: String = reindent_multiline(
                            &format!("{builtin_crate}::future::ready({body_snippet})"),
                            false,
                            Some(4),
                        );

                        let sugg = vec![
                            (async_span, String::new()),
                            (
                                sig.decl.output.span(),
                                format!("impl Future<Output = {signature_snippet}>"),
                            ),
                            (
                                body.value.span,
                                // Wrap the entire snippet in fresh curly braces and indent everything except the first
                                // line by the indentation level of the original body snippet, like this:
                                // {
                                // <indent>     core::future::ready({
                                // <indent>         4
                                // <indent>     }
                                // <indent> }
                                reindent_multiline(
                                    &format!("{{\n{new_body_inner_snippet}\n}}"),
                                    true,
                                    indent_of(cx, body.value.span),
                                ),
                            ),
                        ];
                        diag.help(format!(
                            "a Future can be constructed from the return value with `{builtin_crate}::future::ready`"
                        ));
                        diag.multipart_suggestion(
                                    format!("consider removing the `async` from this function and returning `impl Future<Output = {signature_snippet}>` instead"),
                                    sugg,
                                    app
                                );
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

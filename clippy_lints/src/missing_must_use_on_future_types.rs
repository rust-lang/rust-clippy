use rustc_errors::Applicability;
use rustc_hir::{Impl, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::sym;

use clippy_utils::diagnostics::span_lint_and_then;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for a missing `#[must_use]` attribute on types
    /// that implement `Future`.
    ///
    /// ### Why is this bad?
    /// Futures must be polled for work to be done and it
    /// can be easy to accidentally forget.
    ///
    /// ### Example
    /// ```no_run
    /// struct Foo;
    ///
    /// // impl Future for Foo { ... }
    /// # impl Future for Foo {
    /// #     type Output = ();
    /// #
    /// #     fn poll(
    /// #         self: std::pin::Pin<&mut Self>,
    /// #         _cx: &mut std::task::Context<'_>,
    /// #     ) -> std::task::Poll<Self::Output> {
    /// #         std::task::Poll::Ready(())
    /// #     }
    /// # }
    ///
    /// fn foo() -> Foo { Foo }
    ///
    /// fn main() {
    ///     foo();
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[must_use]
    /// struct Foo;
    ///
    /// // impl Future for Foo { ... }
    /// # impl Future for Foo {
    /// #     type Output = ();
    /// #
    /// #     fn poll(
    /// #         self: std::pin::Pin<&mut Self>,
    /// #         _cx: &mut std::task::Context<'_>,
    /// #     ) -> std::task::Poll<Self::Output> {
    /// #         std::task::Poll::Ready(())
    /// #     }
    /// # }
    ///
    /// fn foo() -> Foo { Foo }
    ///
    /// fn main() {
    ///     foo();
    /// }
    /// ```
    #[clippy::version = "1.86.0"]
    pub MISSING_MUST_USE_ON_FUTURE_TYPES,
    style,
    "missing `#[must_use]` annotation on a type implementing `Future`"
}

declare_lint_pass!(MissingMustUseOnFutureTypes => [MISSING_MUST_USE_ON_FUTURE_TYPES]);

impl<'tcx> LateLintPass<'tcx> for MissingMustUseOnFutureTypes {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        if let ItemKind::Impl(Impl {
            of_trait: Some(trait_ref),
            ..
        }) = item.kind
            && let Some(trait_def_id) = trait_ref.trait_def_id()
            && let Some(future_def_id) = cx.tcx.lang_items().future_trait()
            && trait_def_id == future_def_id
            && let &ty::Adt(adt_def, _) = cx.tcx.type_of(item.owner_id).instantiate_identity().kind()
            && cx.tcx.get_attr(adt_def.did(), sym::must_use).is_none()
        {
            let adt_span = cx.tcx.def_span(adt_def.did());
            span_lint_and_then(
                cx,
                MISSING_MUST_USE_ON_FUTURE_TYPES,
                adt_span,
                "this struct implements `Future` but is missing a `#[must_use]` attribute",
                |diag| {
                    diag.span_suggestion(
                        adt_span.shrink_to_lo(),
                        "add the attribute",
                        "#[must_use]\n",
                        Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}

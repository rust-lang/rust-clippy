use clippy_utils::diagnostics::span_lint;
use rustc_hir::{AssocItemKind, Impl, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `Iterator` implementations not specializing `fold`.
    ///
    /// ### Why is this bad?
    /// Specialize `fold` might provide more performant internal iteration.
    /// Methods relying on `fold` would then be faster as well.
    /// By default, methods that consume the entire iterator rely on it such as `for_each`, `count`...
    ///
    /// ### Example
    /// ```no_run
    /// struct MyIter(u32);
    ///
    /// impl Iterator for MyIter {
    ///     type Item = u32;
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         # todo!()
    ///         // ...
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct MyIter(u32);
    ///
    /// impl Iterator for MyIter {
    ///     type Item = u32;
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         # todo!()
    ///         // ...
    ///     }
    ///     fn fold<B, F>(self, init: B, f: F) -> B
    ///     where
    ///         F: FnMut(B, Self::Item) -> B,
    ///     {
    ///         todo!()
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.77.0"]
    pub MISSING_ITERATOR_FOLD,
    nursery,
    "a missing `Iterator::fold` specialization"
}

declare_lint_pass!(MissingIteratorFold => [MISSING_ITERATOR_FOLD]);

impl LateLintPass<'_> for MissingIteratorFold {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if in_external_macro(cx.sess(), item.span) {
            return;
        }
        if let ItemKind::Impl(Impl {
            of_trait: Some(trait_ref),
            ..
        }) = &item.kind
            && let Some(trait_id) = trait_ref.trait_def_id()
            && cx.tcx.is_diagnostic_item(sym::Iterator, trait_id)
        {
            let has_fold = item
                .expect_impl()
                .items
                .iter()
                .filter(|assoc| matches!(assoc.kind, AssocItemKind::Fn { .. }))
                .any(|assoc| assoc.ident.name.as_str() == "fold");
            if !has_fold {
                span_lint(
                    cx,
                    MISSING_ITERATOR_FOLD,
                    item.span,
                    "you are implementing `Iterator` without specializing its `fold` method",
                );
            }
        }
    }
}

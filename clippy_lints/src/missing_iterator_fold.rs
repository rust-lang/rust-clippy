use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

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

impl LateLintPass<'_> for MissingIteratorFold {}

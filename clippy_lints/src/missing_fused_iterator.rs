use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_lint_allowed;
use clippy_utils::macros::span_is_local;
use clippy_utils::msrvs::Msrv;
use rustc_hir::def_id::DefId;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for publicly reachable iterator types which do not implement
    /// [`FusedIterator`](https://doc.rust-lang.org/std/iter/trait.FusedIterator.html).
    ///
    /// ### Why is this bad?
    /// Implementing `FusedIterator` is a public guarantee that an iterator will keep returning
    /// `None` after it is exhausted. It also allows
    /// [`Iterator::fuse`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fuse)
    /// to optimize its wrapper into a no-op with no performance penalty.
    ///
    /// Not every iterator is fused. This lint should be allowed for iterators which may resume
    /// yielding items after returning `None`.
    ///
    /// ### Example
    /// ```no_run
    /// pub struct Empty;
    ///
    /// impl Iterator for Empty {
    ///     type Item = ();
    ///
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         None
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// pub struct Empty;
    ///
    /// impl Iterator for Empty {
    ///     type Item = ();
    ///
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         None
    ///     }
    /// }
    ///
    /// impl std::iter::FusedIterator for Empty {}
    /// ```
    ///
    /// ### Known problems
    /// Clippy cannot prove that an iterator remains exhausted after returning `None`. In
    /// addition, any positive or negative `FusedIterator` implementation for the type suppresses
    /// this lint, even if its generic bounds do not match every `Iterator` implementation.
    ///
    /// An iterator which is deliberately not fused is best documented with
    /// `#[allow(clippy::missing_fused_iterator)]` and a comment saying why. A negative
    /// implementation, `impl !std::iter::FusedIterator for Empty {}`, also suppresses this lint,
    /// but negative implementations are unstable and only available on nightly.
    #[clippy::version = "1.100.0"]
    pub MISSING_FUSED_ITERATOR,
    pedantic,
    "a publicly reachable iterator type does not implement `FusedIterator`"
}

impl_lint_pass!(MissingFusedIterator => [MISSING_FUSED_ITERATOR]);

pub struct MissingFusedIterator {
    msrv: Msrv,
}

impl MissingFusedIterator {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for MissingFusedIterator {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        if matches!(
            item.kind,
            ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..)
        ) && span_is_local(item.span)
            && cx.effective_visibilities.is_reachable(item.owner_id.def_id)
            && !is_lint_allowed(cx, MISSING_FUSED_ITERATOR, item.hir_id())
            && let Some(iterator_trait) = cx.tcx.lang_items().iterator_trait()
            && let Some(fused_iterator_trait) = cx.tcx.lang_items().fused_iterator_trait()
            && self.msrv.is_stable(cx, fused_iterator_trait)
            && let ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip()
            && cx
                .tcx
                .non_blanket_impls_for_ty(iterator_trait, ty)
                .any(|impl_id| cx.tcx.impl_polarity(impl_id) == ty::ImplPolarity::Positive)
            // Any implementation, including a conditional or negative one, indicates that the
            // `FusedIterator` status of this nominal type has been considered explicitly.
            && cx
                .tcx
                .non_blanket_impls_for_ty(fused_iterator_trait, ty)
                .next()
                .is_none()
        {
            // Point at the `Iterator` implementation which made this type an iterator. It may live
            // far away from the type definition, even in another file.
            let iterator_impl_span = cx
                .tcx
                .non_blanket_impls_for_ty(iterator_trait, ty)
                .filter(|&impl_id| cx.tcx.impl_polarity(impl_id) == ty::ImplPolarity::Positive)
                .filter_map(DefId::as_local)
                .map(|impl_id| cx.tcx.def_span(impl_id))
                .filter(|&span| span_is_local(span))
                .min_by_key(|span| span.lo());

            span_lint_and_then(
                cx,
                MISSING_FUSED_ITERATOR,
                item.span,
                "this publicly reachable type implements `Iterator` but not `FusedIterator`",
                |diag| {
                    let help = "if this iterator remains exhausted after returning `None`, consider implementing `FusedIterator`";
                    if let Some(iterator_impl_span) = iterator_impl_span {
                        diag.span_help(iterator_impl_span, help);
                    } else {
                        diag.help(help);
                    }
                    diag.help(
                        "otherwise, add `#[allow(clippy::missing_fused_iterator)]` with a comment saying why this iterator is not fused",
                    );
                },
            );
        }
    }
}

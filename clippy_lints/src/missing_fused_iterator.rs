use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_lint_allowed;
use clippy_utils::macros::span_is_local;
use clippy_utils::msrvs::Msrv;
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
        if !matches!(
            item.kind,
            ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..)
        ) || !span_is_local(item.span)
            || !cx.effective_visibilities.is_reachable(item.owner_id.def_id)
            || is_lint_allowed(cx, MISSING_FUSED_ITERATOR, item.hir_id())
        {
            return;
        }

        let Some(iterator_trait) = cx.tcx.lang_items().iterator_trait() else {
            return;
        };
        let Some(fused_iterator_trait) = cx.tcx.lang_items().fused_iterator_trait() else {
            return;
        };
        if !self.msrv.is_stable(cx, fused_iterator_trait) {
            return;
        }

        let ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip();
        if !cx
            .tcx
            .non_blanket_impls_for_ty(iterator_trait, ty)
            .any(|impl_id| cx.tcx.impl_polarity(impl_id) == ty::ImplPolarity::Positive)
        {
            return;
        }

        // Any implementation, including a conditional or negative one, indicates that the
        // `FusedIterator` status of this nominal type has been considered explicitly.
        if cx
            .tcx
            .non_blanket_impls_for_ty(fused_iterator_trait, ty)
            .next()
            .is_some()
        {
            return;
        }

        span_lint_and_help(
            cx,
            MISSING_FUSED_ITERATOR,
            item.span,
            "this publicly reachable type implements `Iterator` but not `FusedIterator`",
            None,
            "if this iterator remains exhausted after returning `None`, consider implementing `FusedIterator`",
        );
    }
}

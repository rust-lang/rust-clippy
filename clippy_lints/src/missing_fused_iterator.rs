use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::span_is_local;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{reindent_multiline, snippet, snippet_opt};
use clippy_utils::std_or_core;
use clippy_utils::sugg::DiagExt as _;
use rustc_ast::ImplPolarity;
use rustc_errors::Applicability;
use rustc_hir::attrs::{AttributeKind, LangItem};
use rustc_hir::{Attribute, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_span::Span;
use std::fmt::Write as _;

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
    /// Not every iterator is fused. This lint should be suppressed for iterators which may resume
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
    /// An iterator which is deliberately not fused is best documented by putting
    /// `#[expect(clippy::missing_fused_iterator, reason = "...")]` on its `Iterator`
    /// implementation, or `#[allow(clippy::missing_fused_iterator)]` with a comment saying why
    /// if the MSRV is below 1.81. A negative implementation,
    /// `impl !std::iter::FusedIterator for Empty {}`, also suppresses this lint, but negative
    /// implementations are unstable and only available on nightly.
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
        if let ItemKind::Impl(impl_) = item.kind
            && let Some(of_trait) = impl_.of_trait
            && of_trait.polarity == ImplPolarity::Positive
            && let Some(trait_id) = of_trait.trait_ref.trait_def_id()
            && cx.tcx.is_lang_item(trait_id, LangItem::Iterator)
            && span_is_local(item.span)
            && let self_ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip()
            // Only the nominal type itself counts, not e.g. `impl Iterator for &Foo`.
            && let Some(adt) = self_ty.ty_adt_def()
            && let Some(adt_local_id) = adt.did().as_local()
            && cx.effective_visibilities.is_reachable(adt_local_id)
            && let Some(fused_iterator_trait) = cx.tcx.lang_items().fused_iterator_trait()
            && self.msrv.is_stable(cx, fused_iterator_trait)
            // Any implementation, including a conditional or negative one, indicates that the
            // `FusedIterator` status of this nominal type has been considered explicitly.
            && cx
                .tcx
                .non_blanket_impls_for_ty(fused_iterator_trait, self_ty)
                .next()
                .is_none()
            && let Some(std_or_core) = std_or_core(cx)
        {
            let impl_span = cx.tcx.def_span(item.owner_id);
            let self_ty_snip = if item.span.from_expansion() {
                self_ty.to_string()
            } else {
                snippet(cx, impl_.self_ty.span, "..").into_owned()
            };

            span_lint_and_then(
                cx,
                MISSING_FUSED_ITERATOR,
                impl_span,
                "publicly reachable type with `Iterator` implementation does not implement `FusedIterator`",
                |diag| {
                    diag.span_label(impl_span, "missing `FusedIterator` implementation");
                    diag.note(
                        "public iterators that keep returning `None` once exhausted should implement `FusedIterator` \
                         so that `Iterator::fuse` can become a no-op",
                    );

                    let suppress_attr = if self.msrv.meets(cx, msrvs::LINT_REASONS_STABILIZATION) {
                        "#[expect(clippy::missing_fused_iterator, reason = \"<why this iterator can yield again after returning `None`>\")]"
                    } else {
                        "#[allow(clippy::missing_fused_iterator)] // <why this iterator can yield again after returning `None`>"
                    };
                    let implement_msg = format!("implement `FusedIterator` for `{self_ty_snip}`");
                    let suppress_msg = "if this iterator is intentionally not fused, suppress the lint";

                    let conditional_attrs = if item.span.from_expansion() {
                        None
                    } else {
                        conditional_attrs(cx, item)
                    };
                    if let Some(mut new_impl) = conditional_attrs {
                        let generics = snippet(cx, impl_.generics.span, "");
                        let _ = write!(
                            new_impl,
                            "impl{generics} {std_or_core}::iter::FusedIterator for {self_ty_snip}"
                        );
                        if impl_.generics.has_where_clause_predicates {
                            let where_clause = snippet(cx, impl_.generics.where_clause_span, "");
                            new_impl.push('\n');
                            new_impl.push_str(&reindent_multiline(&where_clause, true, Some(4)));
                            new_impl.push('\n');
                        } else {
                            new_impl.push(' ');
                        }
                        new_impl.push_str("{}");
                        diag.suggest_append_item(
                            cx,
                            item.span,
                            &implement_msg,
                            &new_impl,
                            Applicability::MaybeIncorrect,
                        );
                        diag.suggest_item_with_attr(
                            cx,
                            impl_span,
                            suppress_msg,
                            suppress_attr,
                            Applicability::HasPlaceholders,
                        );
                    } else {
                        diag.help(implement_msg);
                        diag.help(format!("{suppress_msg} with `{suppress_attr}`"));
                    }
                },
            );
        }
    }
}

fn conditional_attrs(cx: &LateContext<'_>, item: &Item<'_>) -> Option<String> {
    let spans: Vec<Span> = cx
        .tcx
        .hir_attrs(item.hir_id())
        .iter()
        .filter_map(|attr| match attr {
            Attribute::Parsed(AttributeKind::CfgTrace(cfgs) | AttributeKind::CfgAttrTrace(cfgs)) => Some(cfgs),
            _ => None,
        })
        .flat_map(|cfgs| cfgs.iter().map(|&(_, span)| span))
        .collect();
    let mut outermost: Vec<Span> = spans
        .iter()
        .copied()
        .filter(|&span| !spans.iter().any(|&other| other != span && other.contains(span)))
        .collect();
    outermost.sort_by_key(|span| span.lo());
    outermost.dedup();

    let mut attrs = String::new();
    for span in outermost {
        attrs.push_str(&snippet_opt(cx, span)?);
        attrs.push('\n');
    }
    Some(attrs)
}

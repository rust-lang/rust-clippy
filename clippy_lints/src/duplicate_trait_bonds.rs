use clippy_utils::diagnostics::span_lint_and_then;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::{Applicability, MultiSpan};
use rustc_hir::{
    GenericBound, GenericBounds, Generics, Item, ItemKind, PolyTraitRef, TraitItem, TraitItemKind, WherePredicateKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::def_id::DefId;
use rustc_span::symbol::Symbol;
use rustc_span::{BytePos, Span};
use std::collections::hash_map::Entry;
use std::convert::TryFrom;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for duplicate trait bounds.
    ///
    /// ### Why is this bad?
    /// Having duplicate trait bounds is redundant and can clutter the code.
    ///
    /// ### Example
    /// ```no_run
    /// struct SomeStruct<T: Clone + Clone> {
    ///     value: T,
    /// }
    ///
    /// impl<T: Send + Sync + Clone + Sync> !Sync for SomeStruct<T> {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct SomeStruct<T: Clone> {
    ///     value: T,
    /// }
    ///
    /// impl<T: Send + Sync + Clone> !Sync for SomeStruct<T> {}
    /// ```
    #[clippy::version = "1.93.0"]
    pub DUPLICATE_TRAIT_BONDS,
    style,
    "duplicate trait bounds"
}

declare_lint_pass!(DuplicateTraitBonds => [DUPLICATE_TRAIT_BONDS]);

impl<'tcx> LateLintPass<'tcx> for DuplicateTraitBonds {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let Some(generics) = item.kind.generics() {
            check_generics(cx, generics);
        }
        match &item.kind {
            ItemKind::Trait(_, _, _, _, _, bounds, _) => lint_bounds(cx, bounds),
            ItemKind::TraitAlias(_, _, bounds) => lint_bounds(cx, bounds),
            _ => {},
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        check_generics(cx, item.generics);
        if let TraitItemKind::Type(bounds, _) = item.kind {
            lint_bounds(cx, bounds);
        }
    }
}

fn check_generics<'tcx>(cx: &LateContext<'tcx>, generics: &'tcx Generics<'tcx>) {
    for predicate in generics.predicates {
        if let WherePredicateKind::BoundPredicate(bound_predicate) = predicate.kind {
            lint_bounds(cx, bound_predicate.bounds);
        }
    }
}

fn lint_bounds<'tcx>(cx: &LateContext<'tcx>, bounds: GenericBounds<'tcx>) {
    let mut duplicates = FxHashMap::default();

    for bound in bounds {
        if let GenericBound::Trait(poly_trait_ref) = bound {
            let key = TraitKey::new(poly_trait_ref);
            let entry = duplicates.entry(key);
            match entry {
                Entry::Vacant(entry) => {
                    entry.insert(TraitGroup {
                        name: trait_name(poly_trait_ref),
                        spans: Vec::new(),
                    });
                },
                Entry::Occupied(mut entry) => {
                    entry.get_mut().spans.push(shrink_span_for_bound(cx, bound.span()));
                },
            }
        }
    }

    // Query instability is OK because it only affects the order of diagnostics.
    #[expect(rustc::potential_query_instability)]
    for group in duplicates.into_values() {
        if group.spans.is_empty() {
            continue;
        }

        let duplicate_spans = group.spans;
        let multi_span = MultiSpan::from_spans(duplicate_spans.clone());

        span_lint_and_then(
            cx,
            DUPLICATE_TRAIT_BONDS,
            multi_span,
            format!("duplicate trait bound `{}` found", group.name),
            move |diag| {
                diag.multipart_suggestion(
                    "consider removing the duplicate",
                    duplicate_spans.into_iter().map(|span| (span, String::new())).collect(),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

fn trait_name(poly_trait_ref: &PolyTraitRef<'_>) -> Symbol {
    poly_trait_ref
        .trait_ref
        .path
        .segments
        .last()
        .map(|segment| segment.ident.name)
        .unwrap_or(Symbol::intern("<unknown>"))
}

struct TraitGroup {
    name: Symbol,
    spans: Vec<Span>,
}

fn shrink_span_for_bound(cx: &LateContext<'_>, span: Span) -> Span {
    if let Ok(prev_source) = cx.tcx.sess.source_map().span_to_prev_source(span) {
        let bytes = prev_source.as_bytes();
        for (idx, ch) in prev_source.char_indices().rev() {
            if ch.is_whitespace() {
                continue;
            }

            if matches!(ch, '+' | ',') {
                let mut len = prev_source.len().saturating_sub(idx);
                let mut start = idx;
                while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                    start -= 1;
                    len += 1;
                }

                if let Ok(len) = u32::try_from(len) {
                    return span.with_lo(span.lo() - BytePos(len));
                }
            }

            break;
        }
    }

    span
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum TraitKey {
    DefId(DefId),
    Name(Symbol),
}

impl TraitKey {
    fn new(poly_trait_ref: &PolyTraitRef<'_>) -> Self {
        if let Some(def_id) = poly_trait_ref.trait_ref.trait_def_id() {
            TraitKey::DefId(def_id)
        } else {
            TraitKey::Name(
                poly_trait_ref
                    .trait_ref
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.name)
                    .unwrap_or(Symbol::intern("<unknown>")),
            )
        }
    }
}

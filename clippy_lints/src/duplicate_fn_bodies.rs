use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::{in_automatically_derived, is_from_proc_macro, tokenize_with_text};
use rustc_data_structures::fx::{FxHashMap, FxHasher};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl, HirId};
use rustc_lexer::TokenKind;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use std::fmt::Write as _;
use std::hash::{Hash as _, Hasher as _};

const MIN_BODY_TOKENS: u32 = 12;

declare_clippy_lint! {
    /// ### What it does
    /// Detects functions whose bodies contain the same Rust token sequence as another function in
    /// the same crate. Whitespace and comments are ignored, but identifier and literal spellings
    /// are preserved. Function bodies from macro expansion and automatically derived code are
    /// ignored.
    ///
    /// ### Why is this bad?
    /// Identical function bodies can increase maintenance burden and allow copies to drift apart.
    /// A bug fix may be applied to one copy but not another. Identical code is not necessarily a
    /// problem, though, and the appropriate refactoring depends on the surrounding API and intent.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn first(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    ///
    /// fn second(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// fn shared(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    ///
    /// fn first(value: i32) -> i32 {
    ///     shared(value)
    /// }
    ///
    /// fn second(value: i32) -> i32 {
    ///     shared(value)
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub DUPLICATE_FN_BODIES,
    nursery,
    "function body is identical to another function body"
}

impl_lint_pass!(DuplicateFnBodies => [DUPLICATE_FN_BODIES]);

#[derive(Default)]
pub struct DuplicateFnBodies {
    bodies: Vec<BodyEntry>,
}

struct BodyEntry {
    hir_id: HirId,
    function_span: Span,
    canonical: Box<str>,
    token_count: u32,
    hash: u64,
}

impl<'tcx> LateLintPass<'tcx> for DuplicateFnBodies {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        function_span: Span,
        def_id: LocalDefId,
    ) {
        let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);
        if !matches!(kind, FnKind::ItemFn(..) | FnKind::Method(..))
            || function_span.from_expansion()
            || body.value.span.from_expansion()
            || in_automatically_derived(cx.tcx, hir_id)
        {
            return;
        }

        if is_from_proc_macro(cx, &(&kind, body, hir_id, function_span)) {
            return;
        }

        let body_span = body.value.span;
        let Some(source) = cx.sess().source_map().span_to_snippet(body_span).ok() else {
            return;
        };

        let canonical = canonical_body(&source);
        if canonical.token_count < MIN_BODY_TOKENS {
            return;
        }

        self.bodies.push(BodyEntry {
            hir_id,
            function_span,
            canonical: canonical.text,
            token_count: canonical.token_count,
            hash: canonical.hash,
        });
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Source order is stable and makes both the primary function and diagnostics deterministic.
        self.bodies
            .sort_by_key(|entry| (entry.function_span.lo(), entry.function_span.hi()));

        let classes = duplicate_classes(
            self.bodies
                .iter()
                .map(|entry| (entry.hash, entry.token_count, entry.canonical.as_ref())),
        );

        for class in classes {
            for &index in &class {
                if let Some(expectation) = cx
                    .tcx
                    .lint_level_spec_at_node(DUPLICATE_FN_BODIES, self.bodies[index].hir_id)
                    .lint_id()
                {
                    cx.fulfill_expectation(expectation);
                }
            }

            let primary = &self.bodies[class[0]];
            for &duplicate_index in &class[1..] {
                let duplicate = &self.bodies[duplicate_index];
                span_lint_hir_and_then(
                    cx,
                    DUPLICATE_FN_BODIES,
                    duplicate.hir_id,
                    duplicate.function_span,
                    "this function has the same body as another function",
                    |diag| {
                        diag.span_note(primary.function_span, "identical function body is defined here");
                        diag.help("consider sharing the implementation if the duplication is unintentional");
                    },
                );
            }
        }
    }
}

struct CanonicalBody {
    text: Box<str>,
    token_count: u32,
    hash: u64,
}

fn canonical_body(source: &str) -> CanonicalBody {
    let mut text = String::with_capacity(source.len());
    let mut token_count = 0;

    for (kind, token, _) in tokenize_with_text(source) {
        if matches!(
            kind,
            TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
        ) {
            continue;
        }

        token_count += 1;
        let _ = write!(text, "{}:", token.len());
        text.push_str(token);
    }

    let mut hasher = FxHasher::default();
    text.hash(&mut hasher);

    CanonicalBody {
        text: text.into_boxed_str(),
        token_count,
        hash: hasher.finish(),
    }
}

fn duplicate_classes<'a>(entries: impl IntoIterator<Item = (u64, u32, &'a str)>) -> Vec<Vec<usize>> {
    let entries: Vec<_> = entries.into_iter().collect();
    let mut candidates: FxHashMap<(u64, u32), Vec<usize>> = FxHashMap::default();
    for (index, (hash, token_count, _)) in entries.iter().enumerate() {
        candidates.entry((*hash, *token_count)).or_default().push(index);
    }

    let mut classes = Vec::new();
    #[expect(rustc::potential_query_instability)]
    for indices in candidates.values() {
        if indices.len() < 2 {
            continue;
        }

        let mut exact: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
        for &index in indices {
            exact.entry(entries[index].2).or_default().push(index);
        }

        #[expect(rustc::potential_query_instability)]
        classes.extend(
            exact
                .into_values()
                .filter_map(|class| if class.len() > 1 { Some(class) } else { None }),
        );
    }

    classes.sort_by_key(|class| class[0]);
    classes
}

#[cfg(test)]
mod tests {
    use super::duplicate_classes;

    #[test]
    fn hash_collisions_still_require_exact_tokens() {
        let first = "4:{1:foo1:}";
        let different = "4:{1:bar1:}";

        let classes = duplicate_classes([(0, 3, first), (0, 3, different), (0, 3, first)]);

        assert_eq!(classes, vec![vec![0, 2]]);
    }
}
